//! Compile-time declaration processing and specialized Rust generation for Kutworld.
//!
//! Generated Rust leaves Rust typing and borrowing checks to `rustc`.

mod generate;
mod model;
mod parse;
mod schedule;

use proc_macro2::TokenStream;

/// Processes an inline world module and generates its storage type.
pub fn world(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let (mut module, world) = parse::parse_world(args, input)?;
    let schedule = schedule::build(&world)?;
    generate::append_world(&mut module, &world, &schedule)?;
    Ok(quote::quote!(#module))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn write_only_is_removed_from_emitted_function() {
        let output = world(
            TokenStream::new(),
            quote! {mod game {
                #[component] struct C;
                #[rule] #[query(has(C))] fn writer(#[write_only] value:&mut C) {}
            }},
        )
        .unwrap()
        .to_string();
        assert!(!output.contains("write_only"));
    }

    #[test]
    fn depends_is_rejected_with_migration_message() {
        let error = world(
            TokenStream::new(),
            quote! {mod game {
                #[component] struct C;
                #[rule] #[query(has(C))] #[depends(input)] fn writer(value:&mut C) {}
            }},
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("`#[depends(...)]` has been removed"));
    }

    #[test]
    fn break_cycle_requires_distinct_valid_inferred_edges() {
        let source = |attribute: proc_macro2::TokenStream| {
            quote! {mod game {
                #[component] struct C;
                #[rule] #[query(has(C))] fn source(value:&mut C) {}
                #[rule] #[query(has(C))] #attribute fn target(value:&C) {}
            }}
        };
        assert!(
            world(
                TokenStream::new(),
                source(quote!(#[break_cycle(source, source)]))
            )
            .unwrap_err()
            .to_string()
            .contains("repeated")
        );
        assert!(
            world(TokenStream::new(), source(quote!(#[break_cycle(missing)])))
                .unwrap_err()
                .to_string()
                .contains("unknown rule")
        );
        assert!(
            world(TokenStream::new(), source(quote!(#[break_cycle()])))
                .unwrap_err()
                .to_string()
                .contains("nonempty name list")
        );
        assert!(
            world(TokenStream::new(), source(quote!(#[break_cycle(target)])))
                .unwrap_err()
                .to_string()
                .contains("itself")
        );
    }

    #[test]
    fn targeted_break_flows_through_codegen_and_generated_docs() {
        let output = world(
            TokenStream::new(),
            quote! {mod game {
                #[component] struct C;
                #[component] struct D;
                #[rule] #[query(has(D))] #[adds(C)] fn a(value:&D) {}
                #[rule] #[query(has(C))] #[adds(D)] #[break_cycle(a)] fn b(value:&C) {}
            }},
        )
        .unwrap()
        .to_string();
        assert!(output.contains("Rule schedule"));
        assert!(output.contains("a` -X→ `b"));
    }

    #[test]
    fn multi_name_break_list_can_consume_both_targeted_edges() {
        let source = quote! {mod game{
            #[component] struct Target;
            #[component] struct FromA;
            #[component] struct FromB;
            #[rule] #[query(has(Target))] #[adds(FromA)] fn a(value:&Target) {}
            #[rule] #[query(has(Target))] #[adds(FromB)] fn b(value:&Target) {}
            #[rule] #[query(has(FromA,FromB))] #[adds(Target)] #[break_cycle(a,b)] fn target(a:&FromA,b:&FromB) {}
        }};
        let output = world(TokenStream::new(), source).unwrap().to_string();
        assert!(output.contains("a` -X→ `target"));
        assert!(output.contains("b` -X→ `target"));
    }

    #[test]
    fn negative_indexed_selector_is_preserved_and_used_for_inference() {
        let source = quote! {mod game{
            #[component] struct Driver; #[component] struct C; #[component] struct D;
            #[indexed_query(C,D)] struct Query;
            #[rule] #[query(has(Driver))] #[adds(C)] fn add(value:&Driver) {}
            #[rule] #[query(has(Driver),not(Query))] fn negative(value:&Driver) {}
        }};
        let (_module, semantic) = parse::parse_world(TokenStream::new(), source).unwrap();
        assert_eq!(
            semantic.rules[1].not,
            vec![syn::Ident::new("Query", proc_macro2::Span::call_site())]
        );
        let schedule = schedule::build(&semantic).unwrap();
        let docs = schedule.rustdoc(&semantic);
        assert!(docs.contains("add` → `negative"));
        assert!(docs.contains("adds C; target observes not(Query) query membership involving C"));
    }

    #[test]
    fn break_cycle_attribute_forms_non_edges_and_duplicate_names_are_rejected() {
        let base = |attrs: proc_macro2::TokenStream| {
            quote! {mod game{
                #[component] struct C;
                #[rule] #[query(has(C))] fn a(value:&mut C) {}
                #[rule] #[query(has(C))] #attrs fn b(value:&C) {}
            }}
        };
        assert!(
            world(TokenStream::new(), base(quote!(#[break_cycle])))
                .unwrap_err()
                .to_string()
                .contains("nonempty name list")
        );
        assert!(
            world(TokenStream::new(), base(quote!(#[break_cycle = a])))
                .unwrap_err()
                .to_string()
                .contains("nonempty name list")
        );
        assert!(
            world(
                TokenStream::new(),
                base(quote!(#[break_cycle(a)] #[break_cycle(a)]))
            )
            .unwrap_err()
            .to_string()
            .contains("duplicate")
        );

        let nonedge = quote! {mod game{
            #[component] struct C;
            #[rule] #[query(has(C))] #[break_cycle(b)] fn a(value:&mut C) {}
            #[rule] #[query(has(C))] fn b(value:&C) {}
        }};
        assert!(
            world(TokenStream::new(), nonedge)
                .unwrap_err()
                .to_string()
                .contains("is not an inferred dependency")
        );

        let duplicate_rules = quote! {mod game{
            #[component] struct C;
            #[rule] #[query(has(C))] fn same(value:&mut C) {}
            #[rule] #[query(has(C))] fn same(value:&C) {}
        }};
        assert!(
            world(TokenStream::new(), duplicate_rules)
                .unwrap_err()
                .to_string()
                .contains("duplicate rule name")
        );
    }

    #[test]
    fn unrelated_acyclic_break_does_not_resolve_another_cycle() {
        let source = quote! {mod game{
            #[component] struct C; #[component] struct D; #[component] struct H;
            #[rule] #[query(has(D))] #[adds(C)] fn a(value:&D) {}
            #[rule] #[query(has(C))] #[adds(D)] fn b(value:&C) {}
            #[rule] #[query(has(H))] fn x(value:&mut H) {}
            #[rule] #[query(has(H))] #[break_cycle(x)] fn y(value:&H) {}
        }};
        let error = world(TokenStream::new(), source).unwrap_err().to_string();
        assert!(error.contains("dependency cycle"));
        assert!(error.contains("no declared #[break_cycle(...)] edge"));
    }

    #[test]
    fn duplicate_or_misplaced_write_only_is_rejected() {
        let duplicate = quote! {mod game{ #[component] struct C; #[rule] #[query(has(C))] fn a(#[write_only] #[write_only] value:&mut C){} }};
        assert!(
            world(TokenStream::new(), duplicate)
                .unwrap_err()
                .to_string()
                .contains("duplicate `#[write_only]`")
        );
        let misplaced = quote! {mod game{ #[component] struct C; #[write_only] #[rule] #[query(has(C))] fn a(value:&mut C){} }};
        assert!(
            world(TokenStream::new(), misplaced)
                .unwrap_err()
                .to_string()
                .contains("only valid on an `&mut Component` rule parameter")
        );
        let nonreference = quote! {mod game{ #[component] struct C; #[rule] #[query(has(C))] fn a(#[write_only] value:C){} }};
        assert!(
            world(TokenStream::new(), nonreference)
                .unwrap_err()
                .to_string()
                .contains("parameters must be `&Component`")
        );
        let not_component = quote! {mod game{ #[component] struct C; #[rule] #[query(has(C))] fn a(#[write_only] value:&mut Missing){} }};
        assert!(
            world(TokenStream::new(), not_component)
                .unwrap_err()
                .to_string()
                .contains("unknown component `Missing`")
        );
    }
}
