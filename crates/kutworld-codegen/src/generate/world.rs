use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub(super) fn generate(world: &World) -> TokenStream {
    let allocator_field = format_ident!("__kutworld_allocator");
    let component_fields = world
        .components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let field = format_ident!("__kutworld_component_{index}");
            let component_name = &component.name;
            quote! { #[allow(dead_code)] #field: ::kutworld::SparseSet<#component_name> }
        });
    let component_initializers = world.components.iter().enumerate().map(|(index, _)| {
        let field = format_ident!("__kutworld_component_{index}");
        quote!(#field: ::kutworld::SparseSet::new())
    });
    quote! {
        pub struct World {
            #[allow(dead_code)]
            #allocator_field: ::kutworld::EntityAllocator,
            #(#component_fields,)*
        }

        impl World {
            pub fn new() -> Self {
                Self {
                    #allocator_field: ::kutworld::EntityAllocator::new(),
                    #(#component_initializers,)*
                }
            }
        }

        impl ::core::default::Default for World {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
