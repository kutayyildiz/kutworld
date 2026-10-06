use syn::Ident;

pub(crate) struct World {
    pub(crate) components: Vec<Component>,
    pub(crate) archetypes: Vec<Archetype>,
}

pub(crate) struct Component {
    pub(crate) name: Ident,
}

pub(crate) struct Archetype {
    pub(crate) name: Ident,
    pub(crate) components: Vec<Ident>,
}
