use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::indexed_query_updates;

pub(super) fn generate(world: &World) -> TokenStream {
    let component_impls = world
        .components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let field = format_ident!("__kutworld_component_{index}");
            let component_name = &component.name;
            let after_add = indexed_query_updates::after_add(world, component);
            let after_remove = indexed_query_updates::after_remove(world, component);
            quote! {
                impl AddComponent<#component_name> for World {
                    fn add_component(
                        &mut self,
                        entity: ::kutworld::Entity,
                        value: #component_name,
                    ) -> bool {
                        if !self.#field.insert(entity, value) {
                            return false;
                        }
                        #after_add
                        true
                    }
                }

                impl RemoveComponent<#component_name> for World {
                    fn remove_component(
                        &mut self,
                        entity: ::kutworld::Entity,
                    ) -> ::core::option::Option<#component_name> {
                        let value = self.#field.remove(entity)?;
                        #after_remove
                        ::core::option::Option::Some(value)
                    }
                }
            }
        });
    let remove_calls = world.components.iter().map(|component| {
        let component_name = &component.name;
        quote! {
            let _ = <Self as RemoveComponent<#component_name>>::remove_component(self, _entity);
        }
    });

    quote! {
        #[allow(dead_code)]
        trait AddComponent<T> {
            fn add_component(
                &mut self,
                entity: ::kutworld::Entity,
                value: T,
            ) -> bool;
        }

        #[allow(dead_code)]
        trait RemoveComponent<T> {
            fn remove_component(
                &mut self,
                entity: ::kutworld::Entity,
            ) -> ::core::option::Option<T>;
        }

        #(#component_impls)*

        impl World {
            #[allow(dead_code)]
            fn __kutworld_despawn(&mut self, _entity: ::kutworld::Entity) {
                #(#remove_calls)*
            }
        }
    }
}
