use crate::schedule::graph::{DependencyGraph, EdgeId};
pub(super) mod v1;
pub(super) struct Resolution {
    pub broken_edges: Vec<EdgeId>,
    pub unresolved_region: Option<Vec<usize>>,
}
pub(super) struct BreakAuthorization {
    pub edge: EdgeId,
    pub source_rule: syn::Ident,
}
pub(super) trait CycleResolver {
    fn name(&self) -> &'static str;
    fn resolve(&self, graph: &DependencyGraph, authorized: &[BreakAuthorization]) -> Resolution;
}
#[derive(Clone, Copy)]
pub(super) enum ResolverKind {
    V1,
}
pub(super) fn resolve(
    kind: ResolverKind,
    graph: &DependencyGraph,
    authorized: &[BreakAuthorization],
) -> (&'static str, Resolution) {
    match kind {
        ResolverKind::V1 => {
            let r = v1::V1;
            (r.name(), r.resolve(graph, authorized))
        }
    }
}
