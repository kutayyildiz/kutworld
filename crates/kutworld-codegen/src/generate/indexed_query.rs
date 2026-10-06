use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn fields(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    world.indexed_queries.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_indexed_query_{index}");
        quote! {
            #[allow(dead_code)]
            #field: ::kutworld::EntitySet
        }
    })
}

pub(super) fn initializers(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    world.indexed_queries.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_indexed_query_{index}");
        quote!(#field: ::kutworld::EntitySet::new())
    })
}
