use syn::Ident;

pub(crate) struct World {
    pub(crate) components: Vec<Component>,
}

pub(crate) struct Component {
    pub(crate) name: Ident,
}
