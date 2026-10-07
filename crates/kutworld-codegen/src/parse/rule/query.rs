use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Meta, Result, Token};

use crate::model::{Rule, World, selector_components};

pub(super) fn parse(attr: &Attribute) -> Result<(Vec<syn::Ident>, Vec<syn::Ident>)> {
    let Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            "rule query must use #[query(has(...), not(...))] syntax",
        ));
    };
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let groups = parser.parse2(list.tokens.clone())?;
    let mut has = None;
    let mut not = None;
    for group in groups {
        let Meta::List(group) = group else {
            return Err(syn::Error::new_spanned(
                group,
                "query selectors must be has(...) or not(...) groups",
            ));
        };
        let Some(name) = group.path.get_ident() else {
            return Err(syn::Error::new_spanned(
                group.path,
                "query selectors must be has(...) or not(...) groups",
            ));
        };
        let parser = Punctuated::<syn::Ident, Token![,]>::parse_terminated;
        let identifiers = parser.parse2(group.tokens)?;
        if identifiers.is_empty() {
            return Err(syn::Error::new_spanned(
                name,
                format!("query group `{name}` must contain at least one name"),
            ));
        }
        let identifiers: Vec<_> = identifiers.into_iter().collect();
        for (index, identifier) in identifiers.iter().enumerate() {
            if identifiers[..index].contains(identifier) {
                return Err(syn::Error::new_spanned(
                    identifier,
                    format!("query selector `{identifier}` is repeated in `{name}`"),
                ));
            }
        }
        match name.to_string().as_str() {
            "has" => {
                if has.replace(identifiers).is_some() {
                    return Err(syn::Error::new_spanned(
                        name,
                        "duplicate `has(...)` query group",
                    ));
                }
            }
            "not" => {
                if not.replace(identifiers).is_some() {
                    return Err(syn::Error::new_spanned(
                        name,
                        "duplicate `not(...)` query group",
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("unsupported query selector group `{name}`"),
                ));
            }
        }
    }
    let Some(has) = has else {
        return Err(syn::Error::new_spanned(
            attr,
            "rule queries require a positive `has(...)` group",
        ));
    };
    Ok((has, not.unwrap_or_default()))
}

pub(super) fn validate(world: &World, rule: &Rule) -> Result<()> {
    let has_requirements = rule
        .has
        .iter()
        .map(|selector| selector_requirements(world, selector))
        .collect::<Result<Vec<_>>>()?;
    let positive_components = unique_components(has_requirements.iter().flatten().cloned());

    for (index, selector) in rule.has.iter().enumerate() {
        let other_components = unique_components(
            has_requirements
                .iter()
                .enumerate()
                .filter(|(other_index, _)| *other_index != index)
                .flat_map(|(_, requirements)| requirements.iter().cloned()),
        );
        if has_requirements[index]
            .iter()
            .all(|component| other_components.contains(component))
        {
            return Err(syn::Error::new_spanned(
                selector,
                format!(
                    "positive selector `{selector}` is redundant in rule `{}`",
                    rule.name
                ),
            ));
        }
    }

    let mut negative_residuals = Vec::new();
    for selector in &rule.not {
        let requirements = selector_requirements(world, selector)?;
        let residual = requirements
            .into_iter()
            .filter(|component| !positive_components.contains(component))
            .collect::<Vec<_>>();
        if residual.is_empty() {
            return Err(syn::Error::new_spanned(
                selector,
                format!(
                    "negative selector `{selector}` contradicts rule `{}` query",
                    rule.name
                ),
            ));
        }
        negative_residuals.push(residual);
    }
    for (index, selector) in rule.not.iter().enumerate() {
        if negative_residuals
            .iter()
            .enumerate()
            .any(|(other_index, other)| {
                other_index != index
                    && other
                        .iter()
                        .all(|component| negative_residuals[index].contains(component))
            })
        {
            return Err(syn::Error::new_spanned(
                selector,
                format!(
                    "negative selector `{selector}` is redundant in rule `{}`",
                    rule.name
                ),
            ));
        }
    }

    for view in rule.accesses.iter().map(|access| &access.component) {
        if !is_component(world, view) {
            return Err(syn::Error::new_spanned(
                view,
                format!("unknown component `{view}` in rule `{}` view", rule.name),
            ));
        }
        if !positive_components.contains(view) {
            return Err(syn::Error::new_spanned(
                view,
                format!(
                    "rule `{}` query does not guarantee component `{view}`",
                    rule.name
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn is_component(world: &World, name: &syn::Ident) -> bool {
    world
        .components
        .iter()
        .any(|component| component.name == *name)
}

fn selector_requirements(world: &World, selector: &syn::Ident) -> Result<Vec<syn::Ident>> {
    if let Some(components) = selector_components(world, selector) {
        return Ok(components);
    }
    Err(syn::Error::new_spanned(
        selector,
        format!("unknown component or indexed query `{selector}`"),
    ))
}

fn unique_components(components: impl IntoIterator<Item = syn::Ident>) -> Vec<syn::Ident> {
    let mut unique = Vec::new();
    for component in components {
        if !unique.contains(&component) {
            unique.push(component);
        }
    }
    unique
}
