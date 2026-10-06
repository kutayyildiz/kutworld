use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Fields, Ident, Item, Result, Token};

use crate::model::{IndexedQuery, World};

pub(super) fn collect(items: &mut [Item], world: &mut World) -> Result<()> {
    for item in items {
        let Item::Struct(item_struct) = item else {
            continue;
        };
        let count = marker_count(&item_struct.attrs);
        if count == 0 {
            continue;
        }
        if item_struct.attrs.iter().any(|attr| {
            matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("component")
        }) {
            return Err(syn::Error::new_spanned(
                &item_struct.ident,
                "a struct cannot be both a component and an indexed query",
            ));
        }
        if count > 1 {
            return Err(syn::Error::new_spanned(
                &item_struct.ident,
                "duplicate #[indexed_query(...)] attribute",
            ));
        }
        if !item_struct.generics.params.is_empty() || item_struct.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                &item_struct.generics,
                "indexed query structs cannot be generic",
            ));
        }
        if !matches!(item_struct.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                &item_struct.fields,
                "indexed query declarations must be unit structs",
            ));
        }

        let attr = item_struct
            .attrs
            .iter()
            .find(|attr| {
                matches!(&attr.style, syn::AttrStyle::Outer)
                    && attr.path().is_ident("indexed_query")
            })
            .expect("marker count was checked");
        let components = parse_components(attr)?;
        world.indexed_queries.push(IndexedQuery {
            name: item_struct.ident.clone(),
            components,
        });
        remove_marker(&mut item_struct.attrs);
    }
    Ok(())
}

pub(super) fn validate(world: &World) -> Result<()> {
    for indexed_query in &world.indexed_queries {
        for component in &indexed_query.components {
            if !world.components.iter().any(|item| item.name == *component) {
                return Err(syn::Error::new_spanned(
                    component,
                    format!(
                        "unknown component `{component}` in indexed query `{}`",
                        indexed_query.name
                    ),
                ));
            }
        }
    }

    for (index, indexed_query) in world.indexed_queries.iter().enumerate() {
        for prior in &world.indexed_queries[..index] {
            if same_requirements(&indexed_query.components, &prior.components) {
                return Err(syn::Error::new_spanned(
                    &indexed_query.name,
                    format!(
                        "indexed query `{}` duplicates the component set of `{}`",
                        indexed_query.name, prior.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn marker_count(attrs: &[Attribute]) -> usize {
    attrs
        .iter()
        .filter(|attr| {
            matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("indexed_query")
        })
        .count()
}

fn parse_components(attr: &Attribute) -> Result<Vec<Ident>> {
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "indexed query requires a nonempty component list, such as #[indexed_query(Position)]",
        ));
    };
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    let components = parser.parse2(list.tokens.clone())?;
    if components.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "indexed query requires at least one component",
        ));
    }
    let components: Vec<_> = components.into_iter().collect();
    for (index, component) in components.iter().enumerate() {
        if components[..index].contains(component) {
            return Err(syn::Error::new_spanned(
                component,
                format!("component `{component}` is repeated in indexed query requirements"),
            ));
        }
    }
    Ok(components)
}

fn remove_marker(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| {
        !matches!(&attr.style, syn::AttrStyle::Outer) || !attr.path().is_ident("indexed_query")
    });
}

fn same_requirements(left: &[Ident], right: &[Ident]) -> bool {
    left.len() == right.len() && left.iter().all(|component| right.contains(component))
}
