use syn::{Attribute, Item, Result};

use crate::model::{Component, World};

pub(super) fn collect(items: &mut [Item], world: &mut World) -> Result<()> {
    for item in items {
        let component = match item {
            Item::Struct(component) if has_marker(&component.attrs) => {
                Some((&mut component.attrs, &component.ident))
            }
            Item::Type(component) if has_marker(&component.attrs) => {
                Some((&mut component.attrs, &component.ident))
            }
            _ => None,
        };
        if let Some((attrs, name)) = component {
            remove_marker(attrs)?;
            world.components.push(Component { name: name.clone() });
        }
    }
    Ok(())
}

fn remove_marker(attrs: &mut Vec<Attribute>) -> Result<()> {
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
            if count > 1 {
                return Err(syn::Error::new_spanned(
                    attr,
                    "duplicate #[component] attribute",
                ));
            }
        } else {
            retained.push(attr);
        }
    }
    *attrs = retained;
    Ok(())
}

fn has_marker(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("component")
    })
}
