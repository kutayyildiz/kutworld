use crate::schedule::{
    graph::{DependencyEdge, DependencyGraph, DependencyReason, EdgeId},
    resolver::{BreakAuthorization, CycleResolver, Resolution},
};

pub(in crate::schedule) struct V1;
impl CycleResolver for V1 {
    fn name(&self) -> &'static str {
        "v1"
    }
    fn resolve(&self, graph: &DependencyGraph, authorized: &[BreakAuthorization]) -> Resolution {
        let mut active = vec![true; graph.edges.len()];
        let mut broken = Vec::new();
        loop {
            let cyclic = graph.cyclic_sccs(&active);
            if cyclic.is_empty() {
                break;
            }
            let region = &cyclic[0];
            let candidates: Vec<_> = graph
                .edges
                .iter()
                .enumerate()
                .filter(|(i, e)| {
                    active[*i]
                        && region.contains(&e.from.0)
                        && region.contains(&e.to.0)
                        && authorized.iter().any(|a| a.edge == EdgeId(*i))
                })
                .map(|(i, _)| EdgeId(i))
                .collect();
            if candidates.is_empty() {
                return Resolution {
                    broken_edges: broken,
                    unresolved_region: Some(region.clone()),
                };
            }
            let mut best = None;
            for id in candidates {
                active[id.0] = false;
                let metrics = cyclic_metrics(graph, &active);
                active[id.0] = true;
                let edge = &graph.edges[id.0];
                let score = (
                    metrics.0,
                    metrics.1,
                    strength(edge).rank(),
                    edge.from.0,
                    edge.to.0,
                );
                if best.as_ref().is_none_or(|(_, s)| score < *s) {
                    best = Some((id, score))
                }
            }
            let id = best.unwrap().0;
            active[id.0] = false;
            broken.push(id);
        }
        Resolution {
            broken_edges: broken,
            unresolved_region: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DependencyStrength {
    ReadWriteBeforeRead,
    WriteBeforeReadWrite,
    WriteBeforeRead,
    StructuralComponent,
    StructuralEntity,
}
impl DependencyStrength {
    // This centralized rank is the v1 policy order, independent of enum declaration order.
    const fn rank(self) -> u8 {
        match self {
            Self::ReadWriteBeforeRead => 0,
            Self::WriteBeforeReadWrite => 1,
            Self::WriteBeforeRead => 2,
            Self::StructuralComponent => 3,
            Self::StructuralEntity => 4,
        }
    }
}
fn strength(edge: &DependencyEdge) -> DependencyStrength {
    edge.reasons
        .iter()
        .map(|r| match r {
            DependencyReason::StructuralComponent { .. } => DependencyStrength::StructuralComponent,
            DependencyReason::StructuralEntity { .. } => DependencyStrength::StructuralEntity,
            DependencyReason::WriteBeforeReadWrite { .. } => {
                DependencyStrength::WriteBeforeReadWrite
            }
            DependencyReason::WriteBeforeRead { .. } => DependencyStrength::WriteBeforeRead,
            DependencyReason::ReadWriteBeforeRead { .. } => DependencyStrength::ReadWriteBeforeRead,
        })
        .max_by_key(|strength| strength.rank())
        .unwrap_or(DependencyStrength::ReadWriteBeforeRead)
}
fn cyclic_metrics(graph: &DependencyGraph, active: &[bool]) -> (usize, usize) {
    let sccs = graph.cyclic_sccs(active);
    let nodes = sccs.iter().map(Vec::len).sum();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            active[*i]
                && sccs
                    .iter()
                    .any(|s| s.contains(&e.from.0) && s.contains(&e.to.0))
        })
        .count();
    (nodes, edges)
}
