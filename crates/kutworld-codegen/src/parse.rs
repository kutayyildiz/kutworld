use proc_macro2::TokenStream;
use syn::{ItemMod, Result};

use crate::model::World;

mod component;
mod indexed_query;
mod initial_entity;
mod rule;

pub(crate) fn parse_world(args: TokenStream, input: TokenStream) -> Result<(ItemMod, World)> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "world does not accept arguments",
        ));
    }
    let mut module = syn::parse2::<ItemMod>(input)?;
    let world = collect_world(&mut module)?;
    Ok((module, world))
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
        indexed_queries: Vec::new(),
        initial_entities: Vec::new(),
        rules: Vec::new(),
    };
    rule::collect(items, &mut world)?;
    initial_entity::collect(items, &mut world)?;
    indexed_query::collect(items, &mut world)?;
    component::collect(items, &mut world)?;
    indexed_query::validate(&world)?;
    initial_entity::validate(&world)?;
    rule::validate(&world)?;
    Ok(world)
}
