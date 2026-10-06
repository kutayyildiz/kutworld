use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::model::World;

pub(super) fn initializers(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    world
        .initial_entities
        .iter()
        .enumerate()
        .map(|(entity_index, initial_entity)| {
            let factory = &initial_entity.name;
            let allocator_field = format_ident!("__kutworld_allocator");
            let entity = format_ident!("__kutworld_initial_entity_{entity_index}");
            let values = initial_entity
                .components
                .iter()
                .enumerate()
                .map(|(component_index, _)| {
                    format_ident!("__kutworld_initial_value_{entity_index}_{component_index}")
                })
                .collect::<Vec<_>>();
            let additions =
                initial_entity
                    .components
                    .iter()
                    .zip(&values)
                    .map(|(component_name, value)| {
                        let component_index = world
                            .components
                            .iter()
                            .position(|component| component.name == *component_name)
                            .expect("initial entity components were validated");
                        let add_method =
                            format_ident!("__kutworld_add_component_{component_index}");
                        quote! { world.#add_method(#entity, #value); }
                    });

            quote! {
                let #entity = world.#allocator_field
                    .allocate()
                    .expect("initial entity allocation exhausted");
                let (#(#values,)*) = self::#factory();
                #(#additions)*
            }
        })
}
