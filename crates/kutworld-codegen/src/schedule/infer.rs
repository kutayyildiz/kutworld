use crate::{
    model::{AccessKind, World, selector_components},
    schedule::graph::{
        ComponentObservation, ComponentStructuralEffect, DependencyEdge, DependencyGraph,
        DependencyReason, EntityStructuralEffect, QueryPolarity, RuleIndex,
    },
};
use std::collections::BTreeMap;

pub(super) fn infer(world: &World) -> DependencyGraph {
    let n = world.rules.len();
    let mut edges: BTreeMap<(usize, usize), Vec<DependencyReason>> = BTreeMap::new();
    let mut membership = vec![Vec::new(); n];
    for (i, r) in world.rules.iter().enumerate() {
        for (selectors, polarity) in [(&r.has, QueryPolarity::Has), (&r.not, QueryPolarity::Not)] {
            for selector in selectors {
                if let Some(components) = selector_components(world, selector) {
                    membership[i].extend(
                        components
                            .into_iter()
                            .map(|component| (component, selector.clone(), polarity)),
                    );
                }
            }
        }
    }
    for a in 0..n {
        for b in 0..n {
            if a == b {
                continue;
            }
            let (ra, rb) = (&world.rules[a], &world.rules[b]);
            for aa in &ra.accesses {
                for ab in &rb.accesses {
                    if aa.component == ab.component {
                        if let Some(reason) = access_reason(aa.kind, ab.kind, &aa.component) {
                            push(&mut edges, a, b, reason)
                        }
                    }
                }
            }
            for (mutated, effect) in ra
                .adds
                .iter()
                .map(|c| (c, ComponentStructuralEffect::Add))
                .chain(
                    ra.removes
                        .iter()
                        .map(|c| (c, ComponentStructuralEffect::Remove)),
                )
            {
                for access in rb
                    .accesses
                    .iter()
                    .filter(|access| access.component == *mutated)
                {
                    push(
                        &mut edges,
                        a,
                        b,
                        DependencyReason::StructuralComponent {
                            component: mutated.clone(),
                            effect,
                            observation: ComponentObservation::Access(access.kind),
                        },
                    );
                }
                for (component, selector, polarity) in &membership[b] {
                    if component == mutated {
                        push(
                            &mut edges,
                            a,
                            b,
                            DependencyReason::StructuralComponent {
                                component: mutated.clone(),
                                effect,
                                observation: ComponentObservation::QueryMembership {
                                    selector: selector.clone(),
                                    polarity: *polarity,
                                },
                            },
                        )
                    }
                }
            }
            if !membership[b].is_empty() {
                if ra.spawns {
                    push(
                        &mut edges,
                        a,
                        b,
                        DependencyReason::StructuralEntity {
                            effect: EntityStructuralEffect::Spawn,
                        },
                    )
                }
                if ra.despawns {
                    push(
                        &mut edges,
                        a,
                        b,
                        DependencyReason::StructuralEntity {
                            effect: EntityStructuralEffect::Despawn,
                        },
                    )
                }
            }
        }
    }
    let edges = edges
        .into_iter()
        .map(|((a, b), reasons)| DependencyEdge {
            from: RuleIndex(a),
            to: RuleIndex(b),
            reasons,
        })
        .collect();
    DependencyGraph { edges, nodes: n }
}
fn access_reason(a: AccessKind, b: AccessKind, c: &syn::Ident) -> Option<DependencyReason> {
    match (a, b) {
        (AccessKind::Write, AccessKind::ReadWrite) => {
            Some(DependencyReason::WriteBeforeReadWrite {
                component: c.clone(),
            })
        }
        (AccessKind::Write, AccessKind::Read) => Some(DependencyReason::WriteBeforeRead {
            component: c.clone(),
        }),
        (AccessKind::ReadWrite, AccessKind::Read) => Some(DependencyReason::ReadWriteBeforeRead {
            component: c.clone(),
        }),
        _ => None,
    }
}
fn push(
    map: &mut BTreeMap<(usize, usize), Vec<DependencyReason>>,
    a: usize,
    b: usize,
    r: DependencyReason,
) {
    let reasons = map.entry((a, b)).or_default();
    if !reasons.contains(&r) {
        reasons.push(r);
    }
}
