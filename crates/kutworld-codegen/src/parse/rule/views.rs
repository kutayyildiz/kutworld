use crate::model::{AccessKind, RuleAccess};
use syn::{FnArg, Result, Signature, Type};

pub(super) fn validate_signature(signature: &Signature) -> Result<()> {
    if signature.asyncness.is_some() || signature.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "rules must be safe synchronous functions",
        ));
    }
    if !signature.generics.params.is_empty() || signature.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &signature.generics,
            "rules cannot be generic",
        ));
    }
    if signature.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            signature,
            "rules cannot have variadic parameters",
        ));
    }
    Ok(())
}

pub(super) fn collect(signature: &mut Signature) -> Result<Vec<RuleAccess>> {
    let mut accesses = Vec::new();
    let mut viewed = Vec::new();
    for input in &mut signature.inputs {
        let FnArg::Typed(argument) = input else {
            return Err(syn::Error::new_spanned(
                input,
                "rule parameters must be component references",
            ));
        };
        let Type::Reference(reference) = argument.ty.as_ref() else {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                "rule parameters must be `&Component` or `&mut Component` views",
            ));
        };
        let Type::Path(path) = reference.elem.as_ref() else {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                "rule views must reference a bare local component type",
            ));
        };
        let Some(component) = path.path.get_ident().filter(|_| path.qself.is_none()) else {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                "rule views must reference a bare local component type",
            ));
        };
        let write_only = argument
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("write_only"))
            .count();
        if write_only > 1 {
            let attr = argument
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("write_only"))
                .nth(1)
                .unwrap();
            return Err(syn::Error::new_spanned(
                attr,
                "duplicate `#[write_only]` attribute",
            ));
        }
        if write_only == 1 && reference.mutability.is_none() {
            let attr = argument
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("write_only"))
                .unwrap();
            return Err(syn::Error::new_spanned(
                attr,
                "`#[write_only]` is only valid on `&mut Component` rule parameters",
            ));
        }
        if let Some(attr) = argument
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("write_only"))
            && !matches!(attr.meta, syn::Meta::Path(_))
        {
            return Err(syn::Error::new_spanned(
                attr,
                "`#[write_only]` does not accept arguments",
            ));
        }
        if viewed.contains(component) {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                format!("component `{component}` has more than one rule view"),
            ));
        }
        viewed.push(component.clone());
        let kind = if reference.mutability.is_some() {
            if write_only == 1 {
                AccessKind::Write
            } else {
                AccessKind::ReadWrite
            }
        } else {
            AccessKind::Read
        };
        accesses.push(RuleAccess {
            component: component.clone(),
            kind,
        });
        argument
            .attrs
            .retain(|attr| !attr.path().is_ident("write_only"));
    }
    Ok(accesses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_annotations_are_interpreted_and_removed() {
        let mut signature: Signature = syn::parse_quote! {
            fn rule(plain: &C, mutable: &mut D, #[write_only] produced: &mut E)
        };
        let accesses = collect(&mut signature).unwrap();
        assert_eq!(
            accesses.iter().map(|a| a.kind).collect::<Vec<_>>(),
            vec![AccessKind::Read, AccessKind::ReadWrite, AccessKind::Write]
        );
        assert!(!quote::quote!(#signature).to_string().contains("write_only"));
    }

    #[test]
    fn write_only_rejects_immutable_and_argument_forms() {
        let mut immutable: Signature = syn::parse_quote! {fn rule(#[write_only] value:&C)};
        assert!(
            collect(&mut immutable)
                .unwrap_err()
                .to_string()
                .contains("only valid on `&mut Component`")
        );
        let mut malformed: Signature =
            syn::parse_quote! {fn rule(#[write_only(read)] value:&mut C)};
        assert!(
            collect(&mut malformed)
                .unwrap_err()
                .to_string()
                .contains("does not accept arguments")
        );
    }
}
