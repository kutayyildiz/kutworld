use syn::Ident;

pub(crate) struct World {
    pub(crate) components: Vec<Component>,
    pub(crate) indexed_queries: Vec<IndexedQuery>,
    pub(crate) initial_entities: Vec<InitialEntity>,
}

pub(crate) struct Component {
    pub(crate) name: Ident,
}

pub(crate) struct IndexedQuery {
    pub(crate) name: Ident,
    pub(crate) components: Vec<Ident>,
}

pub(crate) struct InitialEntity {
    pub(crate) name: Ident,
    pub(crate) components: Vec<Ident>,
}
