use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::{component, indexed_query, initial_entity, mutation};

pub(super) fn generate(world: &World, schedule: &crate::schedule::Schedule) -> TokenStream {
    let allocator_field = format_ident!("__kutworld_allocator");
    let component_fields = component::fields(world);
    let component_initializers = component::initializers(world);
    let indexed_query_fields = indexed_query::fields(world);
    let indexed_query_initializers = indexed_query::initializers(world);
    let initial_entity_initializers = initial_entity::initializers(world);
    let mutation = mutation::generate(world);
    let schedule_docs = schedule.rustdoc(world);
    let world_value = quote! {
        Self {
            #allocator_field: ::kutworld::EntityAllocator::new(),
            #(#component_initializers,)*
            #(#indexed_query_initializers,)*
        }
    };
    let new_body = if world.initial_entities.is_empty() {
        world_value
    } else {
        quote! {
            let mut world = #world_value;
            #(#initial_entity_initializers)*
            world
        }
    };
    quote! {
        #[doc = #schedule_docs]
        pub struct World {
            #[allow(dead_code)]
            #allocator_field: ::kutworld::EntityAllocator,
            #(#component_fields,)*
            #(#indexed_query_fields,)*
        }

        impl World {
            pub fn new() -> Self {
                #new_body
            }

        }

        #mutation

        impl ::core::default::Default for World {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
