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

pub(super) fn collect(signature: &Signature) -> Result<(Vec<syn::Ident>, Vec<syn::Ident>)> {
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    let mut viewed = Vec::new();
    for input in &signature.inputs {
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
        if viewed.contains(component) {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                format!("component `{component}` has more than one rule view"),
            ));
        }
        viewed.push(component.clone());
        if reference.mutability.is_some() {
            writes.push(component.clone());
        } else {
            reads.push(component.clone());
        }
    }
    Ok((reads, writes))
}
