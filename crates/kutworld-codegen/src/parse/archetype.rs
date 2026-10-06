use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Fields, Ident, Item, Result, Token};

use crate::model::{Archetype, World};

pub(super) fn collect(items: &mut [Item], world: &mut World) -> Result<()> {
    for item in items {
        let Item::Struct(item_struct) = item else {
            continue;
        };
        let count = marker_count(&item_struct.attrs);
        if count == 0 {
            continue;
        }
        if item_struct.attrs.iter().any(|attr| {
            matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("component")
        }) {
            return Err(syn::Error::new_spanned(
                &item_struct.ident,
                "a struct cannot be both a component and an archetype",
            ));
        }
        if count > 1 {
            return Err(syn::Error::new_spanned(
                &item_struct.ident,
                "duplicate #[archetype(...)] attribute",
            ));
        }
        if !item_struct.generics.params.is_empty() || item_struct.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                &item_struct.generics,
                "archetype structs cannot be generic",
            ));
        }
        if !matches!(item_struct.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                &item_struct.fields,
                "archetype declarations must be unit structs",
            ));
        }

        let attr = item_struct
            .attrs
            .iter()
            .find(|attr| {
                matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("archetype")
            })
            .expect("marker count was checked");
        let components = parse_components(attr)?;
        world.archetypes.push(Archetype {
            name: item_struct.ident.clone(),
            components,
        });
        remove_marker(&mut item_struct.attrs);
    }
    Ok(())
}

pub(super) fn validate(world: &World) -> Result<()> {
    for archetype in &world.archetypes {
        for component in &archetype.components {
            if !world.components.iter().any(|item| item.name == *component) {
                return Err(syn::Error::new_spanned(
                    component,
                    format!(
                        "unknown component `{component}` in archetype `{}`",
                        archetype.name
                    ),
                ));
            }
        }
    }

    for (index, archetype) in world.archetypes.iter().enumerate() {
        for prior in &world.archetypes[..index] {
            if same_requirements(&archetype.components, &prior.components) {
                return Err(syn::Error::new_spanned(
                    &archetype.name,
                    format!(
                        "archetype `{}` duplicates the component set of `{}`",
                        archetype.name, prior.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn marker_count(attrs: &[Attribute]) -> usize {
    attrs
        .iter()
        .filter(|attr| {
            matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("archetype")
        })
        .count()
}

fn parse_components(attr: &Attribute) -> Result<Vec<Ident>> {
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "archetype requires a nonempty component list, such as #[archetype(Position)]",
        ));
    };
    let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
    let components = parser.parse2(list.tokens.clone())?;
    if components.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "archetype requires at least one component",
        ));
    }
    let components: Vec<_> = components.into_iter().collect();
    for (index, component) in components.iter().enumerate() {
        if components[..index].contains(component) {
            return Err(syn::Error::new_spanned(
                component,
                format!("component `{component}` is repeated in archetype requirements"),
            ));
        }
    }
    Ok(components)
}

fn remove_marker(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| {
        !matches!(&attr.style, syn::AttrStyle::Outer) || !attr.path().is_ident("archetype")
    });
}

fn same_requirements(left: &[Ident], right: &[Ident]) -> bool {
    left.len() == right.len() && left.iter().all(|component| right.contains(component))
}
