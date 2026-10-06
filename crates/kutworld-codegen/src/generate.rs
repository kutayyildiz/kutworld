use crate::model::World;

mod component;
mod indexed_query;
// These generators will be called when generated structural mutation paths are implemented.
#[allow(dead_code)]
mod indexed_query_updates;
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
