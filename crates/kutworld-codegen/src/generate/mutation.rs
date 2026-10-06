use crate::model::World;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::indexed_query_updates;

pub(super) fn methods(world: &World) -> impl Iterator<Item = TokenStream> + '_ {
    let component_methods = world
        .components
        .iter()
        .enumerate()
        .flat_map(|(index, component)| {
            let field = format_ident!("__kutworld_component_{index}");
            let add_method = format_ident!("__kutworld_add_component_{index}");
            let remove_method = format_ident!("__kutworld_remove_component_{index}");
            let component_name = &component.name;
            let after_add = indexed_query_updates::after_add(world, component);
            let after_remove = indexed_query_updates::after_remove(world, component);
            [
                quote! {
                    #[allow(dead_code)]
                    fn #add_method(
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
                },
                quote! {
                    #[allow(dead_code)]
                    fn #remove_method(
                        &mut self,
                        entity: ::kutworld::Entity,
                    ) -> ::core::option::Option<#component_name> {
                        let value = self.#field.remove(entity)?;
                        #after_remove
                        ::core::option::Option::Some(value)
                    }
                },
            ]
        });
    let remove_calls = world.components.iter().enumerate().map(|(index, _)| {
        let remove_method = format_ident!("__kutworld_remove_component_{index}");
        quote! { let _ = self.#remove_method(_entity); }
    });
    let despawn_method = quote! {
        #[allow(dead_code)]
        fn __kutworld_despawn(&mut self, _entity: ::kutworld::Entity) {
            #(#remove_calls)*
        }
    };

    component_methods.chain(std::iter::once(despawn_method))
}
