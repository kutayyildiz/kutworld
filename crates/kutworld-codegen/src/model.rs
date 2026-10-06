use syn::Ident;

pub(crate) struct World {
    pub(crate) components: Vec<Component>,
    pub(crate) indexed_queries: Vec<IndexedQuery>,
    pub(crate) initial_entities: Vec<InitialEntity>,
    pub(crate) rules: Vec<Rule>,
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

pub(crate) struct Rule {
    pub(crate) name: Ident,
    pub(crate) has: Vec<Ident>,
    pub(crate) not: Vec<Ident>,
    pub(crate) reads: Vec<Ident>,
    pub(crate) writes: Vec<Ident>,
    pub(crate) adds: Vec<Ident>,
    pub(crate) removes: Vec<Ident>,
    pub(crate) depends: Vec<Ident>,
    // Consumed by structural execution and scheduling when those stages are implemented.
    #[allow(dead_code)]
    pub(crate) spawns: bool,
    #[allow(dead_code)]
    pub(crate) despawns: bool,
    #[allow(dead_code)]
    pub(crate) serial: bool,
}
