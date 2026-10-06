use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn fields(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    world
        .components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let field = format_ident!("__kutworld_component_{index}");
            let component_name = &component.name;
            quote! { #[allow(dead_code)] #field: ::kutworld::SparseSet<#component_name> }
        })
}

pub(super) fn initializers(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    world.components.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_component_{index}");
        quote!(#field: ::kutworld::SparseSet::new())
    })
}
