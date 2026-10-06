use crate::model::World;

mod component;
mod indexed_query;
mod indexed_query_updates;
mod mutation;
mod world;

pub(crate) fn append_world(module: &mut syn::ItemMod, world: &World) -> syn::Result<()> {
    let generated = world::generate(world);
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
