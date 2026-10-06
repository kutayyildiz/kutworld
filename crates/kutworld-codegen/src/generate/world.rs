use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{component, indexed_query};

pub(super) fn generate(world: &World) -> TokenStream {
    let allocator_field = format_ident!("__kutworld_allocator");
    let component_fields = component::fields(world);
    let component_initializers = component::initializers(world);
    let indexed_query_fields = indexed_query::fields(world);
    let indexed_query_initializers = indexed_query::initializers(world);
    quote! {
        pub struct World {
            #[allow(dead_code)]
            #allocator_field: ::kutworld::EntityAllocator,
            #(#component_fields,)*
            #(#indexed_query_fields,)*
        }

        impl World {
            pub fn new() -> Self {
                Self {
                    #allocator_field: ::kutworld::EntityAllocator::new(),
                    #(#component_initializers,)*
                    #(#indexed_query_initializers,)*
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
