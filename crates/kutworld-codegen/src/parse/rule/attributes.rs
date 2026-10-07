use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Result, Token};

pub(super) fn has_attribute(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| is_marker(attr, name))
}

pub(super) fn one_attribute<'a>(
    attrs: &'a [Attribute],
    name: &str,
) -> Result<Option<&'a Attribute>> {
    let mut found = None;
    for attr in attrs.iter().filter(|attr| is_marker(attr, name)) {
        if found.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                format!("duplicate `#[{name}]` attribute"),
            ));
        }
        found = Some(attr);
    }
    Ok(found)
}

pub(super) fn require_path_attribute(attr: &Attribute, expected: &str) -> Result<()> {
    if matches!(attr.meta, syn::Meta::Path(_)) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            attr,
            format!("`{expected}` does not accept arguments"),
        ))
    }
}

pub(super) fn parse_name_attribute(attrs: &[Attribute], name: &str) -> Result<Vec<syn::Ident>> {
    let Some(attr) = one_attribute(attrs, name)? else {
        return Ok(Vec::new());
    };
    let syn::Meta::List(list) = &attr.meta else {
        return Err(syn::Error::new_spanned(
            attr,
            format!("`#[{name}(...)]` requires a nonempty name list"),
        ));
    };
    let parser = Punctuated::<syn::Ident, Token![,]>::parse_terminated;
    let names = parser.parse2(list.tokens.clone())?;
    if names.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            format!("`#[{name}(...)]` requires a nonempty name list"),
        ));
    }
    let names: Vec<_> = names.into_iter().collect();
    for (index, item) in names.iter().enumerate() {
        if names[..index].contains(item) {
            return Err(syn::Error::new_spanned(
                item,
                format!("`{item}` is repeated in `#[{name}(...)]`"),
            ));
        }
    }
    Ok(names)
}

pub(super) fn parse_flag_attribute(attrs: &[Attribute], name: &str) -> Result<bool> {
    let Some(attr) = one_attribute(attrs, name)? else {
        return Ok(false);
    };
    require_path_attribute(attr, &format!("#[{name}]"))?;
    Ok(true)
}

pub(super) fn remove_rule_attributes(attrs: &mut Vec<Attribute>) {
    const NAMES: &[&str] = &[
        "rule",
        "query",
        "adds",
        "removes",
        "spawns",
        "despawns",
        "break_cycle",
        "serial",
    ];
    attrs.retain(|attr| !NAMES.iter().any(|name| is_marker(attr, name)));
}

fn is_marker(attr: &Attribute, name: &str) -> bool {
    matches!(&attr.style, syn::AttrStyle::Outer) && attr.path().is_ident(name)
}
