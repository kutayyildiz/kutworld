use super::*;
use crate::model::{AccessKind, Component, Rule, RuleAccess};
use crate::schedule::{
    graph::{
        ComponentObservation, ComponentStructuralEffect, DependencyEdge, DependencyGraph,
        DependencyReason, EdgeId, EntityStructuralEffect, QueryPolarity, RuleIndex,
    },
    resolver::BreakAuthorization,
    resolver::v1::V1,
    resolver::{CycleResolver, Resolution},
    validate::{validate_choices, validate_resolution},
};
use syn::Ident;

fn id(name: &str) -> Ident {
    Ident::new(name, proc_macro2::Span::call_site())
}
fn rule(
    name: &str,
    has: &[&str],
    accesses: &[(&str, AccessKind)],
    adds: &[&str],
    breaks: &[&str],
) -> Rule {
    Rule {
        name: id(name),
        has: has.iter().map(|s| id(s)).collect(),
        not: Vec::new(),
        accesses: accesses
            .iter()
            .map(|(c, k)| RuleAccess {
                component: id(c),
                kind: *k,
            })
            .collect(),
        adds: adds.iter().map(|s| id(s)).collect(),
        removes: Vec::new(),
        break_cycle: breaks.iter().map(|s| id(s)).collect(),
        spawns: false,
        despawns: false,
        serial: false,
    }
}
fn world(rules: Vec<Rule>, components: &[&str]) -> World {
    World {
        components: components
            .iter()
            .map(|name| Component { name: id(name) })
            .collect(),
        indexed_queries: Vec::new(),
        initial_entities: Vec::new(),
        rules,
    }
}

#[test]
fn access_inference_orders_only_strict_same_component_classes() {
    let w = world(
        vec![
            rule("write", &[], &[("C", AccessKind::Write)], &[], &[]),
            rule("rw", &[], &[("C", AccessKind::ReadWrite)], &[], &[]),
            rule("read", &[], &[("C", AccessKind::Read)], &[], &[]),
            rule("other", &[], &[("D", AccessKind::Read)], &[], &[]),
        ],
        &["C", "D"],
    );
    let g = infer::infer(&w);
    assert!(g.edge_id(0, 1).is_some());
    assert!(g.edge_id(0, 2).is_some());
    assert!(g.edge_id(1, 2).is_some());
    assert!(g.edge_id(0, 0).is_none());
    assert!(g.edge_id(0, 3).is_none());
    assert!(g.edge_id(1, 0).is_none());
    assert!(g.edge_id(2, 1).is_none());
}

#[test]
fn all_equal_access_classes_and_unrelated_components_are_unordered() {
    for kind in [AccessKind::Read, AccessKind::ReadWrite, AccessKind::Write] {
        let graph = infer::infer(&world(
            vec![
                rule("a", &[], &[("C", kind)], &[], &[]),
                rule("b", &[], &[("C", kind)], &[], &[]),
            ],
            &["C"],
        ));
        assert!(
            graph.edges.is_empty(),
            "equal {kind:?} accesses must remain unordered"
        );
    }
}

#[test]
fn structural_addition_orders_positive_and_negative_query_membership() {
    let mut w = world(
        vec![
            rule("add", &[], &[], &["C"], &[]),
            rule("positive", &["C"], &[], &[], &[]),
            rule("negative", &[], &[], &[], &[]),
        ],
        &["C"],
    );
    w.rules[2].not.push(id("C"));
    let g = infer::infer(&w);
    assert!(g.edge_id(0, 1).is_some());
    assert!(g.edge_id(0, 2).is_some());
}

#[test]
fn targeted_break_resolves_cycle_and_order_is_stable() {
    let w = world(
        vec![
            rule("a", &["D"], &[], &["C"], &[]),
            rule("b", &["C"], &[], &["D"], &["a"]),
            rule("free", &[], &[], &[], &[]),
        ],
        &["C", "D"],
    );
    let schedule = build(&w).unwrap();
    assert_eq!(
        schedule.order.iter().map(|r| r.0).collect::<Vec<_>>(),
        vec![1, 0, 2]
    );
    assert_eq!(schedule.broken.len(), 1);
    assert_eq!(schedule.broken[0].source_rule, id("a"));
    let docs = schedule.rustdoc(&w);
    assert!(docs.contains("a` -X→ `b`"));
    assert!(docs.contains("Final execution order"));
    let positions = schedule
        .order
        .iter()
        .enumerate()
        .map(|(pos, rule)| (rule.0, pos))
        .collect::<std::collections::BTreeMap<_, _>>();
    let broken = schedule
        .broken
        .iter()
        .map(|edge| edge.id)
        .collect::<Vec<_>>();
    for (edge_id, edge) in schedule.graph.edges.iter().enumerate() {
        if !broken.contains(&EdgeId(edge_id)) {
            assert!(positions[&edge.from.0] < positions[&edge.to.0]);
        }
    }
    for _ in 0..5 {
        let repeat = build(&w).unwrap();
        assert_eq!(
            repeat.order.iter().map(|rule| rule.0).collect::<Vec<_>>(),
            vec![1, 0, 2]
        );
        assert_eq!(
            repeat.broken.iter().map(|edge| edge.id).collect::<Vec<_>>(),
            broken
        );
    }
}

#[test]
fn cycles_without_authorization_report_reasons() {
    let w = world(
        vec![
            rule("a", &["D"], &[], &["C"], &[]),
            rule("b", &["C"], &[], &["D"], &[]),
        ],
        &["C", "D"],
    );
    let error = match build(&w) {
        Ok(_) => panic!("expected unresolved cycle"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("dependency cycle"));
    assert!(error.contains("adds C; target observes has(C) query membership involving C"));
    assert!(error.contains("no declared #[break_cycle(...)] edge"));
}

#[test]
fn equal_access_classes_remain_unordered() {
    let mut w = world(
        vec![
            rule("a", &[], &[("C", AccessKind::Write)], &[], &[]),
            rule("b", &[], &[("C", AccessKind::Write)], &[], &[]),
        ],
        &["C"],
    );
    w.rules[0].serial = true;
    let g = infer::infer(&w);
    assert!(g.edges.is_empty());
    let schedule = build(&w).unwrap();
    assert_eq!(
        schedule.order.iter().map(|r| r.0).collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn v1_strength_order_is_pairwise_and_uses_strongest_aggregated_reason() {
    let reason = |strength: usize| match strength {
        0 => DependencyReason::ReadWriteBeforeRead { component: id("C") },
        1 => DependencyReason::WriteBeforeReadWrite { component: id("C") },
        2 => DependencyReason::WriteBeforeRead { component: id("C") },
        3 => structural_component(
            "C",
            ComponentStructuralEffect::Add,
            ComponentObservation::Access(AccessKind::Read),
        ),
        _ => structural_entity(EntityStructuralEffect::Spawn),
    };
    for left in 0..5 {
        for right in 0..5 {
            let graph = DependencyGraph {
                nodes: 3,
                edges: vec![
                    edge(0, 1, reason(left)),
                    edge(1, 2, reason(right)),
                    edge(2, 0, reason(4)),
                ],
            };
            let auth = vec![
                BreakAuthorization {
                    edge: EdgeId(0),
                    source_rule: id("r0"),
                },
                BreakAuthorization {
                    edge: EdgeId(1),
                    source_rule: id("r1"),
                },
            ];
            let result = V1.resolve(&graph, &auth);
            assert_eq!(
                result.broken_edges[0],
                if left <= right { EdgeId(0) } else { EdgeId(1) },
                "strength pair ({left}, {right})"
            );
        }
    }

    let graph = DependencyGraph {
        nodes: 3,
        edges: vec![
            DependencyEdge {
                from: RuleIndex(0),
                to: RuleIndex(1),
                reasons: vec![reason(0), reason(3)],
            },
            edge(1, 2, reason(1)),
            edge(2, 0, reason(4)),
        ],
    };
    let auth = vec![
        BreakAuthorization {
            edge: EdgeId(0),
            source_rule: id("r0"),
        },
        BreakAuthorization {
            edge: EdgeId(1),
            source_rule: id("r1"),
        },
    ];
    assert_eq!(V1.resolve(&graph, &auth).broken_edges[0], EdgeId(1));
}

#[test]
fn shared_chokepoint_wins_before_semantic_strength() {
    let world = world(
        (0..4)
            .map(|i| rule(&format!("r{i}"), &[], &[], &[], &[]))
            .collect(),
        &[],
    );
    let graph = DependencyGraph {
        nodes: 4,
        edges: vec![
            edge(0, 1, structural_entity(EntityStructuralEffect::Spawn)),
            edge(
                1,
                2,
                DependencyReason::WriteBeforeReadWrite { component: id("C") },
            ),
            edge(
                2,
                0,
                DependencyReason::WriteBeforeRead { component: id("C") },
            ),
            edge(
                1,
                3,
                DependencyReason::ReadWriteBeforeRead { component: id("D") },
            ),
            edge(
                3,
                0,
                DependencyReason::WriteBeforeRead { component: id("D") },
            ),
        ],
    };
    let shared = graph.edge_id(0, 1).unwrap();
    let local = graph.edge_id(2, 0).unwrap();
    let auth = vec![
        BreakAuthorization {
            edge: local,
            source_rule: id("r2"),
        },
        BreakAuthorization {
            edge: shared,
            source_rule: id("r0"),
        },
    ];
    let result = V1.resolve(&graph, &auth);
    assert_eq!(result.broken_edges, vec![shared]);
    let error = validate_resolution(&world, &graph, &result, &auth)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unused"));
}

fn structural_entity(effect: EntityStructuralEffect) -> DependencyReason {
    DependencyReason::StructuralEntity { effect }
}
fn structural_component(
    component: &str,
    effect: ComponentStructuralEffect,
    observation: ComponentObservation,
) -> DependencyReason {
    DependencyReason::StructuralComponent {
        component: id(component),
        effect,
        observation,
    }
}
fn edge(from: usize, to: usize, reason: DependencyReason) -> DependencyEdge {
    DependencyEdge {
        from: RuleIndex(from),
        to: RuleIndex(to),
        reasons: vec![reason],
    }
}

#[test]
fn tied_candidates_are_stable_across_edge_and_authorization_insertion_order() {
    let endpoints = [(0, 1), (1, 2), (2, 0)];
    let run = |pairs: &[(usize, usize)]| {
        let graph = DependencyGraph {
            nodes: 3,
            edges: pairs
                .iter()
                .map(|(a, b)| {
                    edge(
                        *a,
                        *b,
                        DependencyReason::ReadWriteBeforeRead { component: id("C") },
                    )
                })
                .collect(),
        };
        let auth = pairs
            .iter()
            .enumerate()
            .rev()
            .map(|(i, (a, _))| BreakAuthorization {
                edge: EdgeId(i),
                source_rule: id(&format!("r{a}")),
            })
            .collect::<Vec<_>>();
        let result = V1.resolve(&graph, &auth);
        let selected = &graph.edges[result.broken_edges[0].0];
        (selected.from.0, selected.to.0)
    };
    let reversed = [endpoints[2], endpoints[1], endpoints[0]];
    assert_eq!(run(&endpoints), (0, 1));
    assert_eq!(run(&reversed), (0, 1));
}

#[test]
fn v1_recomputes_sccs_until_multiple_cycles_are_broken() {
    let w = world(
        vec![
            rule("a", &["D"], &[], &["C"], &[]),
            rule("b", &["C"], &[], &["D"], &["a"]),
            rule("c", &["F"], &[], &["E"], &[]),
            rule("d", &["E"], &[], &["F"], &["c"]),
        ],
        &["C", "D", "E", "F"],
    );
    let graph = infer::infer(&w);
    let auth = vec![
        BreakAuthorization {
            edge: graph.edge_id(0, 1).unwrap(),
            source_rule: id("a"),
        },
        BreakAuthorization {
            edge: graph.edge_id(2, 3).unwrap(),
            source_rule: id("c"),
        },
    ];
    let result = V1.resolve(&graph, &auth);
    assert_eq!(result.broken_edges.len(), 2);
    assert!(result.unresolved_region.is_none());
}

#[test]
fn overlapping_scc_splits_after_each_break_and_reports_later_unresolved_region() {
    let graph = DependencyGraph {
        nodes: 5,
        edges: vec![
            edge(0, 1, structural_entity(EntityStructuralEffect::Spawn)),
            edge(
                1,
                2,
                DependencyReason::WriteBeforeRead { component: id("A") },
            ),
            edge(
                2,
                0,
                DependencyReason::ReadWriteBeforeRead { component: id("A") },
            ),
            edge(0, 3, structural_entity(EntityStructuralEffect::Spawn)),
            edge(
                3,
                4,
                DependencyReason::WriteBeforeRead { component: id("B") },
            ),
            edge(
                4,
                0,
                DependencyReason::ReadWriteBeforeRead { component: id("B") },
            ),
        ],
    };
    let auth = vec![
        BreakAuthorization {
            edge: graph.edge_id(1, 2).unwrap(),
            source_rule: id("r1"),
        },
        BreakAuthorization {
            edge: graph.edge_id(3, 4).unwrap(),
            source_rule: id("r3"),
        },
    ];
    let result = V1.resolve(&graph, &auth);
    assert_eq!(result.broken_edges.len(), 2);
    assert!(result.unresolved_region.is_none());
    let mut active = vec![true; graph.edges.len()];
    for edge in &result.broken_edges {
        active[edge.0] = false
    }
    assert!(graph.cyclic_sccs(&active).is_empty());
    let only_first = V1.resolve(&graph, &auth[..1]);
    assert_eq!(only_first.broken_edges.len(), 1);
    assert!(only_first.unresolved_region.is_some());
}

#[test]
fn inference_aggregates_structural_and_data_provenance_on_one_edge() {
    let w = world(
        vec![
            rule("writer", &[], &[("C", AccessKind::Write)], &["C"], &[]),
            rule("reader", &["C"], &[("C", AccessKind::ReadWrite)], &[], &[]),
        ],
        &["C"],
    );
    let graph = infer::infer(&w);
    let edge = &graph.edges[graph.edge_id(0, 1).unwrap().0];
    assert_eq!(edge.reasons.len(), 3);
    assert!(edge.reasons.iter().any(|r| matches!(r,DependencyReason::StructuralComponent{component,effect:ComponentStructuralEffect::Add,observation:ComponentObservation::Access(AccessKind::ReadWrite)} if *component==id("C"))));
    assert!(edge.reasons.iter().any(|r| matches!(r,DependencyReason::StructuralComponent{component,effect:ComponentStructuralEffect::Add,observation:ComponentObservation::QueryMembership{selector,polarity:QueryPolarity::Has}} if *component==id("C")&&*selector==id("C"))));
    assert!(edge.reasons.iter().any(
        |r| matches!(r,DependencyReason::WriteBeforeReadWrite{component} if *component==id("C"))
    ));
    let schedule = build(&w).unwrap();
    let docs = schedule.rustdoc(&w);
    assert!(docs.contains("adds C; target reads and writes C"));
    assert!(docs.contains("adds C; target observes has(C) query membership involving C"));
    assert!(docs.contains("Write -> ReadWrite"));
}

#[test]
fn add_remove_spawn_despawn_are_distinct_coalesced_reasons() {
    let mut source = rule("source", &[], &[], &["C"], &[]);
    source.removes.push(id("C"));
    source.spawns = true;
    source.despawns = true;
    let target = rule("target", &["C"], &[("C", AccessKind::Read)], &[], &[]);
    let graph = infer::infer(&world(vec![source, target], &["C"]));
    let edge = &graph.edges[graph.edge_id(0, 1).unwrap().0];
    assert!(edge.reasons.contains(&structural_component(
        "C",
        ComponentStructuralEffect::Add,
        ComponentObservation::Access(AccessKind::Read)
    )));
    assert!(edge.reasons.contains(&structural_component(
        "C",
        ComponentStructuralEffect::Remove,
        ComponentObservation::Access(AccessKind::Read)
    )));
    assert!(edge.reasons.contains(&structural_component(
        "C",
        ComponentStructuralEffect::Add,
        ComponentObservation::QueryMembership {
            selector: id("C"),
            polarity: QueryPolarity::Has
        }
    )));
    assert!(edge.reasons.contains(&structural_component(
        "C",
        ComponentStructuralEffect::Remove,
        ComponentObservation::QueryMembership {
            selector: id("C"),
            polarity: QueryPolarity::Has
        }
    )));
    assert!(
        edge.reasons
            .contains(&structural_entity(EntityStructuralEffect::Spawn))
    );
    assert!(
        edge.reasons
            .contains(&structural_entity(EntityStructuralEffect::Despawn))
    );
    assert_eq!(edge.reasons.len(), 6);
}

#[test]
fn direct_break_does_not_remove_an_indirect_path_and_redundancy_still_applies() {
    let w = world(
        (0..5)
            .map(|i| rule(&format!("r{i}"), &[], &[], &[], &[]))
            .collect(),
        &[],
    );
    let graph = DependencyGraph {
        nodes: 5,
        edges: vec![
            edge(
                0,
                2,
                DependencyReason::WriteBeforeRead { component: id("C") },
            ),
            edge(
                0,
                1,
                DependencyReason::WriteBeforeRead { component: id("C") },
            ),
            edge(
                1,
                2,
                DependencyReason::WriteBeforeRead { component: id("C") },
            ),
            edge(
                2,
                0,
                DependencyReason::WriteBeforeRead { component: id("C") },
            ),
            edge(
                3,
                4,
                DependencyReason::WriteBeforeRead { component: id("D") },
            ),
            edge(
                4,
                3,
                DependencyReason::WriteBeforeRead { component: id("D") },
            ),
        ],
    };
    let authorized_pairs = [(0, 2), (2, 0), (3, 4)];
    let ids = authorized_pairs.map(|(from, to)| graph.edge_id(from, to).unwrap());
    let auth = ids
        .iter()
        .zip(authorized_pairs)
        .map(|(edge, (from, _))| BreakAuthorization {
            edge: *edge,
            source_rule: id(&format!("r{from}")),
        })
        .collect::<Vec<_>>();
    let resolution = Resolution {
        broken_edges: ids.to_vec(),
        unresolved_region: None,
    };
    let mut active = vec![true; graph.edges.len()];
    for edge in &resolution.broken_edges {
        active[edge.0] = false
    }
    assert!(active[graph.edge_id(0, 1).unwrap().0]);
    assert!(active[graph.edge_id(1, 2).unwrap().0]);
    assert!(
        validate_resolution(&w, &graph, &resolution, &auth)
            .unwrap_err()
            .to_string()
            .contains("redundant")
    );
}

#[test]
fn common_validation_rejects_unused_and_consumed_redundant_breaks() {
    let w = world(
        vec![rule("a", &[], &[], &[], &[]), rule("b", &[], &[], &[], &[])],
        &[],
    );
    let graph = DependencyGraph {
        nodes: 2,
        edges: vec![
            DependencyEdge {
                from: RuleIndex(0),
                to: RuleIndex(1),
                reasons: vec![structural_entity(EntityStructuralEffect::Spawn)],
            },
            DependencyEdge {
                from: RuleIndex(1),
                to: RuleIndex(0),
                reasons: vec![structural_entity(EntityStructuralEffect::Despawn)],
            },
        ],
    };
    let auth = vec![
        BreakAuthorization {
            edge: EdgeId(0),
            source_rule: id("a"),
        },
        BreakAuthorization {
            edge: EdgeId(1),
            source_rule: id("b"),
        },
    ];
    let resolution = Resolution {
        broken_edges: vec![EdgeId(0), EdgeId(1)],
        unresolved_region: None,
    };
    assert!(
        validate_resolution(&w, &graph, &resolution, &auth)
            .unwrap_err()
            .to_string()
            .contains("redundant")
    );

    let acyclic = DependencyGraph {
        nodes: 2,
        edges: graph.edges[..1].to_vec(),
    };
    let unused = Resolution {
        broken_edges: vec![],
        unresolved_region: None,
    };
    assert!(
        validate_resolution(&w, &acyclic, &unused, &auth[..1])
            .unwrap_err()
            .to_string()
            .contains("unused")
    );
}

#[test]
fn common_validation_rejects_invalid_resolver_edge_before_graph_mutation() {
    let w = world(
        vec![rule("a", &[], &[], &[], &[]), rule("b", &[], &[], &[], &[])],
        &[],
    );
    let graph = DependencyGraph {
        nodes: 2,
        edges: vec![edge(0, 1, structural_entity(EntityStructuralEffect::Spawn))],
    };
    let invalid = Resolution {
        broken_edges: vec![EdgeId(99)],
        unresolved_region: None,
    };
    assert!(
        validate_choices(&w, &graph, &invalid, &[])
            .unwrap_err()
            .to_string()
            .contains("invalid or duplicate edge")
    );
}

#[test]
fn v1_greedy_overlapping_cycles_can_consume_a_later_redundant_break() {
    let world = world(
        vec![
            rule("r0", &[], &[], &[], &[]),
            rule("r1", &[], &[], &[], &[]),
            rule("r2", &[], &[], &[], &[]),
        ],
        &[],
    );
    let reason = || DependencyReason::ReadWriteBeforeRead { component: id("C") };
    let graph = DependencyGraph {
        nodes: 3,
        edges: vec![
            edge(0, 1, reason()),
            edge(0, 2, reason()),
            edge(1, 0, reason()),
            edge(1, 2, reason()),
            edge(2, 0, reason()),
            edge(2, 1, reason()),
        ],
    };
    let auth = [(0, 1), (1, 0), (1, 2), (2, 0)]
        .into_iter()
        .map(|(from, to)| BreakAuthorization {
            edge: graph.edge_id(from, to).unwrap(),
            source_rule: id(&format!("r{from}")),
        })
        .collect::<Vec<_>>();
    let resolution = V1.resolve(&graph, &auth);
    assert_eq!(resolution.broken_edges.len(), 4);
    assert!(resolution.unresolved_region.is_none());
    assert!(
        validate_resolution(&world, &graph, &resolution, &auth)
            .unwrap_err()
            .to_string()
            .contains("redundant")
    );
}

#[test]
fn indexed_and_entity_membership_effects_are_inferred() {
    let mut w = world(
        vec![
            rule("add", &[], &[], &["C"], &[]),
            rule("indexed", &["HasCD"], &[], &[], &[]),
            rule("spawn", &[], &[], &[], &[]),
            rule("entities", &["C"], &[], &[], &[]),
        ],
        &["C", "D"],
    );
    w.indexed_queries.push(crate::model::IndexedQuery {
        name: id("HasCD"),
        components: vec![id("C"), id("D")],
    });
    w.rules[2].spawns = true;
    let graph = infer::infer(&w);
    assert!(graph.edge_id(0, 1).is_some());
    assert!(graph.edge_id(2, 1).is_some());
    assert!(graph.edge_id(2, 3).is_some());
}

#[test]
fn removal_despawn_negative_index_membership_and_serial_effects() {
    let mut w = world(
        vec![
            rule("remove", &[], &[], &[], &[]),
            rule("negative_index", &["D"], &[], &[], &[]),
            rule("despawn", &[], &[], &[], &[]),
            rule("entity_query", &["E"], &[], &[], &[]),
            rule("unrelated", &["D"], &[], &[], &[]),
            rule("serial", &[], &[("C", AccessKind::Write)], &[], &[]),
        ],
        &["C", "D", "E", "X"],
    );
    w.indexed_queries.push(crate::model::IndexedQuery {
        name: id("NeedsCE"),
        components: vec![id("C"), id("E")],
    });
    w.rules[0].removes.push(id("C"));
    w.rules[1].not.push(id("NeedsCE"));
    w.rules[2].despawns = true;
    w.rules[5].serial = true;
    let graph = infer::infer(&w);
    assert!(graph.edge_id(0, 1).is_some());
    assert!(graph.edges[graph.edge_id(0,1).unwrap().0].reasons.iter().any(|r|matches!(r,DependencyReason::StructuralComponent{component,..} if *component==id("C"))));
    assert!(graph.edge_id(2, 3).is_some());
    assert!(graph.edge_id(0, 4).is_none());
    assert!(graph.edge_id(2, 4).is_some());
    assert_eq!(w.rules[1].not.len(), 1);
    assert_eq!(w.rules[1].not[0], id("NeedsCE"));
}
