use crate::model::{Component, World};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[cfg(test)]
mod tests;

#[derive(Clone)]
struct Candidate {
    indexed_query_index: usize,
    remaining: Vec<usize>,
}

pub(super) fn after_add(world: &World, component: &Component) -> TokenStream {
    let candidates = world
        .indexed_queries
        .iter()
        .enumerate()
        .filter_map(|(indexed_query_index, indexed_query)| {
            if !indexed_query.components.contains(&component.name) {
                return None;
            }
            let remaining = indexed_query
                .components
                .iter()
                .filter(|required| **required != component.name)
                .map(|required| {
                    world
                        .components
                        .iter()
                        .position(|item| item.name == *required)
                        .expect("indexed query requirements were validated")
                })
                .collect();
            Some(Candidate {
                indexed_query_index,
                remaining,
            })
        })
        .collect();
    generate_decisions(world, candidates)
}

pub(super) fn after_remove(world: &World, component: &Component) -> TokenStream {
    let affected = world
        .indexed_queries
        .iter()
        .enumerate()
        .filter(|(_, indexed_query)| indexed_query.components.contains(&component.name))
        .map(|(index, _)| {
            let field = format_ident!("__kutworld_indexed_query_{index}");
            quote!(self.#field.remove(entity);)
        });
    quote!(#(#affected)*)
}

fn generate_decisions(world: &World, candidates: Vec<Candidate>) -> TokenStream {
    let (completed, pending): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|candidate| candidate.remaining.is_empty());
    let completed_updates = completed.iter().map(|candidate| {
        let field = format_ident!("__kutworld_indexed_query_{}", candidate.indexed_query_index);
        quote!(self.#field.insert(entity);)
    });
    if pending.is_empty() {
        return quote!(#(#completed_updates)*);
    }
    let component_counts = world.components.iter().enumerate().map(|(index, _)| {
        let count = pending
            .iter()
            .filter(|candidate| candidate.remaining.contains(&index))
            .count();
        (index, count)
    });
    let mut selected = None;
    let mut selected_count = 0;
    for (index, count) in component_counts {
        if count > selected_count {
            selected = Some(index);
            selected_count = count;
        }
    }
    let selected = selected.expect("pending candidates contain a remaining component");

    let component_field = format_ident!("__kutworld_component_{selected}");
    let (present_candidates, absent_candidates): (Vec<_>, Vec<_>) = pending
        .into_iter()
        .partition(|candidate| candidate.remaining.contains(&selected));
    let present_candidates = present_candidates
        .into_iter()
        .map(|mut candidate| {
            candidate.remaining.retain(|index| *index != selected);
            candidate
        })
        .chain(absent_candidates.iter().cloned())
        .collect();
    let present = generate_decisions(world, present_candidates);
    let absent = generate_decisions(world, absent_candidates);
    quote! {
        #(#completed_updates)*
        if self.#component_field.contains(entity) {
            #present
        } else {
            #absent
        }
    }
}
