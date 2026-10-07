use crate::{
    model::World,
    schedule::{
        graph::{DependencyGraph, topological_order},
        resolver::{BreakAuthorization, Resolution},
    },
};

pub(super) fn authorize(
    world: &World,
    graph: &DependencyGraph,
) -> syn::Result<Vec<BreakAuthorization>> {
    let mut authorized = Vec::new();
    for (to, rule) in world.rules.iter().enumerate() {
        for name in &rule.break_cycle {
            let Some(from) = world.rules.iter().position(|r| r.name == *name) else {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("unknown rule `{name}` in `#[break_cycle(...)]`"),
                ));
            };
            if from == to {
                return Err(syn::Error::new_spanned(
                    name,
                    "a rule cannot break a dependency from itself",
                ));
            }
            let Some(id) = graph.edge_id(from, to) else {
                return Err(syn::Error::new_spanned(
                    name,
                    format!("`{name} -> {}` is not an inferred dependency", rule.name),
                ));
            };
            authorized.push(BreakAuthorization {
                edge: id,
                source_rule: name.clone(),
            });
        }
    }
    Ok(authorized)
}

pub(super) fn validate_resolution(
    world: &World,
    graph: &DependencyGraph,
    resolution: &Resolution,
    authorized: &[BreakAuthorization],
) -> syn::Result<()> {
    validate_choices(world, graph, resolution, authorized)?;
    let mut active = vec![true; graph.edges.len()];
    let mut seen = Vec::new();
    for id in &resolution.broken_edges {
        active[id.0] = false;
        seen.push(*id);
    }
    if !graph.cyclic_sccs(&active).is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "cycle resolver left cyclic dependencies",
        ));
    }
    for authorization in authorized {
        if !seen.contains(&authorization.edge) {
            return Err(syn::Error::new_spanned(
                &authorization.source_rule,
                format!(
                    "unused `#[break_cycle({})]`: dependency is not needed by the selected resolution",
                    authorization.source_rule
                ),
            ));
        }
    }
    for id in &seen {
        active[id.0] = true;
        if topological_order(world.rules.len(), graph, &active).is_some() {
            let span = &authorized
                .iter()
                .find(|a| a.edge == *id)
                .expect("selected edges are authorized")
                .source_rule;
            return Err(syn::Error::new_spanned(
                span,
                format!(
                    "`#[break_cycle({span})]` is redundant; restoring this dependency keeps the graph acyclic"
                ),
            ));
        }
        active[id.0] = false;
    }
    Ok(())
}

pub(super) fn validate_choices(
    world: &World,
    graph: &DependencyGraph,
    resolution: &Resolution,
    authorized: &[BreakAuthorization],
) -> syn::Result<()> {
    let mut seen = Vec::new();
    for id in &resolution.broken_edges {
        if id.0 >= graph.edges.len() || seen.contains(id) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cycle resolver returned an invalid or duplicate edge",
            ));
        }
        let edge = &graph.edges[id.0];
        if !authorized.iter().any(|candidate| candidate.edge == *id) {
            return Err(syn::Error::new_spanned(
                &world.rules[edge.to.0].name,
                "cycle resolver selected an unauthorized edge",
            ));
        }
        seen.push(*id);
    }
    if let Some(region) = &resolution.unresolved_region {
        if region.is_empty() || region.iter().any(|node| *node >= graph.nodes) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "cycle resolver returned an invalid unresolved region",
            ));
        }
    }
    Ok(())
}

pub(super) fn unresolved_cycle(
    world: &World,
    graph: &DependencyGraph,
    active: &[bool],
    region: &[usize],
) -> syn::Error {
    let cycle = graph.cycle(active, region);
    let mut message = String::from("dependency cycle:\n");
    for id in cycle {
        let edge = &graph.edges[id.0];
        message.push_str(&format!(
            "    {} -> {}\n",
            world.rules[edge.from.0].name, world.rules[edge.to.0].name
        ));
        for reason in &edge.reasons {
            message.push_str(&format!("      {}\n", reason.describe()));
        }
    }
    message.push_str("no declared #[break_cycle(...)] edge can resolve this region");
    syn::Error::new_spanned(&world.rules[region[0]].name, message)
}
