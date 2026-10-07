//! Semantic dependency inference, cycle resolution, validation, and stable ordering.

mod graph;
mod infer;
mod resolver;
#[cfg(test)]
mod tests;
mod validate;

use crate::model::World;
use graph::{DependencyGraph, RuleIndex};

pub(crate) struct Schedule {
    order: Vec<RuleIndex>,
    graph: DependencyGraph,
    broken: Vec<graph::BrokenDependency>,
    resolver: &'static str,
}

impl Schedule {
    pub(crate) fn rustdoc(&self, world: &World) -> String {
        let mut lines = vec![
            "# Rule schedule".to_owned(),
            "".to_owned(),
            format!("Resolver: `{}`", self.resolver),
            "".to_owned(),
            "## Inferred dependencies".to_owned(),
            "".to_owned(),
        ];
        if self.graph.edges.is_empty() {
            lines.push("- None".to_owned());
        }
        for edge in &self.graph.edges {
            let reasons = edge
                .reasons
                .iter()
                .map(|r| r.describe())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "- `{}` → `{}`: {reasons}",
                world.rules[edge.from.0].name, world.rules[edge.to.0].name
            ));
        }
        lines.push(String::new());
        lines.push("## Explicit cycle breaks".to_owned());
        lines.push(String::new());
        if self.broken.is_empty() {
            lines.push("- None".to_owned());
        }
        for broken in &self.broken {
            let reasons = &self.graph.edges[broken.id.0].reasons;
            let reasons = reasons
                .iter()
                .map(|reason| reason.describe())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "- `{}` -X→ `{}`: {reasons} (authorized by `#[break_cycle({})]`)",
                world.rules[broken.from.0].name, world.rules[broken.to.0].name, broken.source_rule
            ));
        }
        lines.push(String::new());
        lines.push("## Final execution order".to_owned());
        lines.push(String::new());
        lines.push("This is the computed schedule; rule execution is not implemented.".to_owned());
        lines.push(String::new());
        for (i, rule) in self.order.iter().enumerate() {
            lines.push(format!("{}. `{}`", i + 1, world.rules[rule.0].name));
        }
        lines.join("\n")
    }
}

pub(crate) fn build(world: &World) -> syn::Result<Schedule> {
    build_with_resolver(world, resolver::ResolverKind::V1)
}

fn build_with_resolver(
    world: &World,
    resolver_kind: resolver::ResolverKind,
) -> syn::Result<Schedule> {
    let graph = infer::infer(world);
    let authorized = validate::authorize(world, &graph)?;
    let (resolver, resolution) = resolver::resolve(resolver_kind, &graph, &authorized);
    validate::validate_choices(world, &graph, &resolution, &authorized)?;
    let mut active = vec![true; graph.edges.len()];
    for edge in &resolution.broken_edges {
        active[edge.0] = false;
    }
    if let Some(region) = &resolution.unresolved_region {
        if !graph.cyclic_sccs(&active).iter().any(|scc| scc == region) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cycle resolver returned a region that is not cyclic in the remaining graph",
            ));
        }
        return Err(validate::unresolved_cycle(world, &graph, &active, region));
    }
    validate::validate_resolution(world, &graph, &resolution, &authorized)?;
    let order = graph::topological_order(world.rules.len(), &graph, &active).ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "resolved rule graph is cyclic",
        )
    })?;
    let broken = resolution
        .broken_edges
        .iter()
        .map(|id| {
            let e = &graph.edges[id.0];
            graph::BrokenDependency {
                id: *id,
                from: e.from,
                to: e.to,
                source_rule: authorized
                    .iter()
                    .find(|a| a.edge == *id)
                    .expect("broken edges authorized")
                    .source_rule
                    .clone(),
            }
        })
        .collect();
    Ok(Schedule {
        order,
        graph,
        broken,
        resolver,
    })
}
