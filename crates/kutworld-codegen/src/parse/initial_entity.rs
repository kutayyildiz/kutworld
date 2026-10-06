use syn::{Attribute, Item, Result, Type};

use crate::model::{InitialEntity, World};

pub(super) fn collect(items: &mut [Item], world: &mut World) -> Result<()> {
    for item in items {
        let Item::Fn(function) = item else {
            continue;
        };
        if !has_marker(&function.attrs) {
            continue;
        }

        remove_marker(&mut function.attrs)?;
        let signature = &function.sig;
        if !signature.inputs.is_empty() {
            return Err(syn::Error::new_spanned(
                &signature.inputs,
                "#[initial_entity] functions cannot take parameters",
            ));
        }
        if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                &signature.generics,
                "#[initial_entity] functions cannot be generic",
            ));
        }
        if signature.asyncness.is_some() || signature.unsafety.is_some() {
            return Err(syn::Error::new_spanned(
                signature,
                "#[initial_entity] functions cannot be async or unsafe",
            ));
        }
        let syn::ReturnType::Type(_, return_type) = &signature.output else {
            return Err(syn::Error::new_spanned(
                signature,
                "#[initial_entity] functions must declare a tuple return type",
            ));
        };
        let Type::Tuple(tuple) = return_type.as_ref() else {
            return Err(syn::Error::new_spanned(
                return_type,
                "#[initial_entity] functions must return a tuple of components",
            ));
        };

        let mut components = Vec::with_capacity(tuple.elems.len());
        for element in &tuple.elems {
            let Type::Path(path) = element else {
                return Err(syn::Error::new_spanned(
                    element,
                    "initial entity tuple elements must name local components",
                ));
            };
            let Some(component) = path.path.get_ident().filter(|_| path.qself.is_none()) else {
                return Err(syn::Error::new_spanned(
                    element,
                    "initial entity tuple elements must use bare local component names",
                ));
            };
            if components
                .iter()
                .any(|existing: &syn::Ident| existing == component)
            {
                return Err(syn::Error::new_spanned(
                    element,
                    format!("duplicate initial entity component `{component}`"),
                ));
            }
            components.push(component.clone());
        }

        world.initial_entities.push(InitialEntity {
            name: function.sig.ident.clone(),
            components,
        });
    }
    Ok(())
}

pub(super) fn validate(world: &World) -> Result<()> {
    for initial_entity in &world.initial_entities {
        for component in &initial_entity.components {
            if !world
                .components
                .iter()
                .any(|declared| declared.name == *component)
            {
                return Err(syn::Error::new_spanned(
                    component,
                    format!(
                        "initial entity `{}` references unknown local component `{component}`",
                        initial_entity.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn remove_marker(attrs: &mut Vec<Attribute>) -> Result<()> {
    let mut count = 0;
    let mut retained = Vec::new();
    for attr in attrs.drain(..) {
        if matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("initial_entity") {
            count += 1;
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "#[initial_entity] does not accept arguments",
                ));
            }
            if count > 1 {
                return Err(syn::Error::new_spanned(
                    attr,
                    "duplicate #[initial_entity] attribute",
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
        matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident("initial_entity")
    })
}
