use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn fields(world: &World) -> TokenStream {
    let fields = world.archetypes.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_archetype_{index}");
        quote! {
            #[allow(dead_code)]
            #field: ::kutworld::EntitySet
        }
    });
    quote!(#(#fields,)*)
}

pub(super) fn initializers(world: &World) -> TokenStream {
    let initializers = world.archetypes.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_archetype_{index}");
        quote!(#field: ::kutworld::EntitySet::new())
    });
    quote!(#(#initializers,)*)
}
