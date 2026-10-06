use proc_macro2::TokenStream;
use syn::{Attribute, Item, ItemMod, Result};

use crate::generate;
use crate::model::{Component, World};

pub(crate) fn parse_world(args: TokenStream, input: TokenStream) -> Result<ItemMod> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "world does not accept arguments",
        ));
    }
    let mut module = syn::parse2::<ItemMod>(input)?;
    let world = collect_world(&mut module)?;
    generate::append_world(&mut module, &world)?;
    Ok(module)
}

fn collect_world(module: &mut ItemMod) -> Result<World> {
    let world_name = module.ident.clone();
    let Some((_, items)) = module.content.as_mut() else {
        return Err(syn::Error::new_spanned(
            world_name,
            "world modules must be inline",
        ));
    };

    let mut world = World {
        components: Vec::new(),
    };
    for item in items {
        let component = match item {
            Item::Struct(component) if has_marker(&component.attrs, "component") => {
                Some((&mut component.attrs, &component.ident))
            }
            Item::Type(component) if has_marker(&component.attrs, "component") => {
                Some((&mut component.attrs, &component.ident))
            }
            _ => None,
        };
        if let Some((attrs, name)) = component {
            remove_component_marker(attrs)?;
            world.components.push(Component { name: name.clone() });
        }
    }
    Ok(world)
}

fn remove_component_marker(attrs: &mut Vec<Attribute>) -> Result<()> {
    let mut count = 0;
    let mut retained = Vec::new();
    for attr in attrs.drain(..) {
        if matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("component") {
            count += 1;
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "#[component] does not accept arguments",
                ));
            }
        } else {
            retained.push(attr);
        }
    }
    *attrs = retained;
    if count > 1 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "duplicate #[component] attribute",
        ));
    }
    Ok(())
}

fn has_marker(attrs: &[Attribute], marker: &str) -> bool {
    attrs
        .iter()
        .any(|attr| matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident(marker))
}
