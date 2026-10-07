use crate::model::World;

mod component;
mod indexed_query;
mod indexed_query_updates;
mod initial_entity;
mod mutation;
mod world;

pub(crate) fn append_world(
    module: &mut syn::ItemMod,
    world: &World,
    schedule: &crate::schedule::Schedule,
) -> syn::Result<()> {
    let generated = world::generate(world, schedule);
    let generated_items = syn::parse2::<syn::File>(generated)
        .map_err(|error| syn::Error::new_spanned(&module.ident, error))?
        .items;
    let (_, items) = module
        .content
        .as_mut()
        .expect("world was checked as inline");
    items.extend(generated_items);
    Ok(())
}
