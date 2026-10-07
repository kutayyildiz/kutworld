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
    pub(crate) accesses: Vec<RuleAccess>,
    pub(crate) adds: Vec<Ident>,
    pub(crate) removes: Vec<Ident>,
    pub(crate) break_cycle: Vec<Ident>,
    pub(crate) spawns: bool,
    pub(crate) despawns: bool,
    #[allow(dead_code)]
    pub(crate) serial: bool,
}

pub(crate) fn selector_components(world: &World, selector: &Ident) -> Option<Vec<Ident>> {
    if world
        .components
        .iter()
        .any(|component| component.name == *selector)
    {
        Some(vec![selector.clone()])
    } else {
        world
            .indexed_queries
            .iter()
            .find(|query| query.name == *selector)
            .map(|query| query.components.clone())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessKind {
    Read,
    ReadWrite,
    Write,
}

#[derive(Debug)]
pub(crate) struct RuleAccess {
    pub(crate) component: Ident,
    pub(crate) kind: AccessKind,
}
