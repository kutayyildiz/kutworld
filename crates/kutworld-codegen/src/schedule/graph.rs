use crate::model::AccessKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct RuleIndex(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct EdgeId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DependencyReason {
    WriteBeforeReadWrite {
        component: syn::Ident,
    },
    WriteBeforeRead {
        component: syn::Ident,
    },
    ReadWriteBeforeRead {
        component: syn::Ident,
    },
    StructuralComponent {
        component: syn::Ident,
        effect: ComponentStructuralEffect,
        observation: ComponentObservation,
    },
    StructuralEntity {
        effect: EntityStructuralEffect,
    },
}
impl DependencyReason {
    pub(super) fn describe(&self) -> String {
        match self {
            Self::WriteBeforeReadWrite { component } => format!("{component}: Write -> ReadWrite"),
            Self::WriteBeforeRead { component } => format!("{component}: Write -> Read"),
            Self::ReadWriteBeforeRead { component } => format!("{component}: ReadWrite -> Read"),
            Self::StructuralComponent {
                component,
                effect,
                observation,
            } => {
                let effect = match effect {
                    ComponentStructuralEffect::Add => "adds",
                    ComponentStructuralEffect::Remove => "removes",
                };
                let observation = match observation {
                    ComponentObservation::Access(AccessKind::Read) => {
                        format!("target reads {component}")
                    }
                    ComponentObservation::Access(AccessKind::ReadWrite) => {
                        format!("target reads and writes {component}")
                    }
                    ComponentObservation::Access(AccessKind::Write) => {
                        format!("target writes {component}")
                    }
                    ComponentObservation::QueryMembership {
                        selector,
                        polarity: QueryPolarity::Has,
                    } => format!(
                        "target observes has({selector}) query membership involving {component}"
                    ),
                    ComponentObservation::QueryMembership {
                        selector,
                        polarity: QueryPolarity::Not,
                    } => format!(
                        "target observes not({selector}) query membership involving {component}"
                    ),
                };
                format!("{effect} {component}; {observation}")
            }
            Self::StructuralEntity {
                effect: EntityStructuralEffect::Spawn,
            } => "spawns an entity; target query membership may change".into(),
            Self::StructuralEntity {
                effect: EntityStructuralEffect::Despawn,
            } => "despawns an entity; target query membership may change".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ComponentStructuralEffect {
    Add,
    Remove,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntityStructuralEffect {
    Spawn,
    Despawn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QueryPolarity {
    Has,
    Not,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ComponentObservation {
    Access(AccessKind),
    QueryMembership {
        selector: syn::Ident,
        polarity: QueryPolarity,
    },
}
#[derive(Clone, Debug)]
pub(super) struct DependencyEdge {
    pub from: RuleIndex,
    pub to: RuleIndex,
    pub reasons: Vec<DependencyReason>,
}
pub(super) struct DependencyGraph {
    pub edges: Vec<DependencyEdge>,
    pub nodes: usize,
}
#[derive(Clone)]
pub(super) struct BrokenDependency {
    pub id: EdgeId,
    pub from: RuleIndex,
    pub to: RuleIndex,
    pub source_rule: syn::Ident,
}

impl DependencyGraph {
    pub(super) fn edge_id(&self, from: usize, to: usize) -> Option<EdgeId> {
        self.edges
            .iter()
            .position(|e| e.from.0 == from && e.to.0 == to)
            .map(EdgeId)
    }
    pub(super) fn sccs(&self, active: &[bool]) -> Vec<Vec<usize>> {
        fn visit(v: usize, adj: &[Vec<usize>], seen: &mut [bool], out: &mut Vec<usize>) {
            if seen[v] {
                return;
            }
            seen[v] = true;
            for &w in &adj[v] {
                visit(w, adj, seen, out)
            }
            out.push(v);
        }
        fn assign(v: usize, rev: &[Vec<usize>], seen: &mut [bool], c: &mut Vec<usize>) {
            if seen[v] {
                return;
            }
            seen[v] = true;
            c.push(v);
            for &w in &rev[v] {
                assign(w, rev, seen, c)
            }
        }
        let mut adj = vec![Vec::new(); self.nodes];
        let mut rev = vec![Vec::new(); self.nodes];
        for (i, e) in self.edges.iter().enumerate() {
            if active[i] {
                adj[e.from.0].push(e.to.0);
                rev[e.to.0].push(e.from.0)
            }
        }
        let mut seen = vec![false; self.nodes];
        let mut order = Vec::new();
        for v in 0..self.nodes {
            visit(v, &adj, &mut seen, &mut order)
        }
        seen.fill(false);
        let mut out = Vec::new();
        for v in order.into_iter().rev() {
            if seen[v] {
                continue;
            }
            let mut c = Vec::new();
            assign(v, &rev, &mut seen, &mut c);
            c.sort_unstable();
            out.push(c)
        }
        out.sort_by_key(|c| c[0]);
        out
    }
    pub(super) fn cyclic_sccs(&self, active: &[bool]) -> Vec<Vec<usize>> {
        self.sccs(active)
            .into_iter()
            .filter(|c| {
                c.len() > 1
                    || self
                        .edges
                        .iter()
                        .enumerate()
                        .any(|(i, e)| active[i] && e.from.0 == c[0] && e.to.0 == c[0])
            })
            .collect()
    }
    pub(super) fn cycle(&self, active: &[bool], scc: &[usize]) -> Vec<EdgeId> {
        let mut adj = vec![Vec::new(); self.nodes];
        for (i, e) in self.edges.iter().enumerate() {
            if active[i] && scc.contains(&e.from.0) && scc.contains(&e.to.0) {
                adj[e.from.0].push((e.to.0, EdgeId(i)))
            }
        }
        for a in &mut adj {
            a.sort_by_key(|x| x.0)
        }
        fn dfs(
            v: usize,
            adj: &[Vec<(usize, EdgeId)>],
            state: &mut [u8],
            nodes: &mut Vec<usize>,
            path: &mut Vec<EdgeId>,
        ) -> Option<Vec<EdgeId>> {
            state[v] = 1;
            nodes.push(v);
            for &(w, id) in &adj[v] {
                if state[w] == 1 {
                    let at = nodes.iter().position(|n| *n == w).unwrap();
                    let mut result = path[at..].to_vec();
                    result.push(id);
                    return Some(result);
                }
                if state[w] == 0 {
                    path.push(id);
                    if let Some(c) = dfs(w, adj, state, nodes, path) {
                        return Some(c);
                    }
                    path.pop();
                }
            }
            nodes.pop();
            state[v] = 2;
            None
        }
        let mut state = vec![0; self.nodes];
        let mut nodes = Vec::new();
        let mut path = Vec::new();
        for &v in scc {
            if state[v] == 0
                && let Some(c) = dfs(v, &adj, &mut state, &mut nodes, &mut path)
            {
                return c;
            }
        }
        Vec::new()
    }
}

pub(super) fn topological_order(
    nodes: usize,
    graph: &DependencyGraph,
    active: &[bool],
) -> Option<Vec<RuleIndex>> {
    let mut indegree = vec![0; nodes];
    let mut adj = vec![Vec::new(); nodes];
    for (i, e) in graph.edges.iter().enumerate() {
        if active[i] {
            adj[e.from.0].push(e.to.0);
            indegree[e.to.0] += 1
        }
    }
    let mut order = Vec::new();
    let mut used = vec![false; nodes];
    while order.len() < nodes {
        let n = (0..nodes).find(|&i| !used[i] && indegree[i] == 0)?;
        used[n] = true;
        order.push(RuleIndex(n));
        for &v in &adj[n] {
            indegree[v] -= 1
        }
    }
    Some(order)
}
