use crate::model::World;
use quote::{format_ident, quote};

pub(crate) fn append_world(module: &mut syn::ItemMod, world: &World) -> syn::Result<()> {
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
    let generated = quote! {
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
    };
    let generated_items = syn::parse2::<syn::File>(generated)
        .map_err(|error| syn::Error::new_spanned(&module.ident, error))?
        .items;
    let (_, items) = module
        .content
        .as_mut()
        .expect("world was checked as inline");
    items.extend(generated_items);
    Ok(())
}
