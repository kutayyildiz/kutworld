use proc_macro2::TokenStream;
use syn::{ItemMod, Result};

use crate::generate;
use crate::model::World;

mod archetype;
mod component;

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
        archetypes: Vec::new(),
    };
    archetype::collect(items, &mut world)?;
    component::collect(items, &mut world)?;
    archetype::validate(&world)?;
    Ok(world)
}
