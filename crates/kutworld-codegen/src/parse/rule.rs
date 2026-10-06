use syn::{Item, Result};

use crate::model::{Rule, World};

mod attributes;
mod dependencies;
mod query;
mod views;

pub(super) fn collect(items: &mut [Item], world: &mut World) -> Result<()> {
    for item in items {
        let Item::Fn(function) = item else {
            continue;
        };
        if !attributes::has_attribute(&function.attrs, "rule") {
            continue;
        }
        if attributes::has_attribute(&function.attrs, "initial_entity") {
            return Err(syn::Error::new_spanned(
                &function.sig.ident,
                "a function cannot be both a rule and an initial entity factory",
            ));
        }

        let rule_attr =
            attributes::one_attribute(&function.attrs, "rule")?.expect("rule marker checked");
        attributes::require_path_attribute(rule_attr, "#[rule]")?;
        views::validate_signature(&function.sig)?;
        let query_attr = attributes::one_attribute(&function.attrs, "query")?.ok_or_else(|| {
            syn::Error::new_spanned(
                &function.sig.ident,
                "rules require one #[query(...)] attribute",
            )
        })?;
        let (has, not) = query::parse(query_attr)?;
        let (reads, writes) = views::collect(&function.sig)?;
        let adds = attributes::parse_name_attribute(&function.attrs, "adds")?;
        let removes = attributes::parse_name_attribute(&function.attrs, "removes")?;
        let depends = attributes::parse_name_attribute(&function.attrs, "depends")?;
        let spawns = attributes::parse_flag_attribute(&function.attrs, "spawns")?;
        let despawns = attributes::parse_flag_attribute(&function.attrs, "despawns")?;
        let serial = attributes::parse_flag_attribute(&function.attrs, "serial")?;
        attributes::remove_rule_attributes(&mut function.attrs);
        world.rules.push(Rule {
            name: function.sig.ident.clone(),
            has,
            not,
            reads,
            writes,
            adds,
            removes,
            depends,
            spawns,
            despawns,
            serial,
        });
    }
    Ok(())
}

pub(super) fn validate(world: &World) -> Result<()> {
    for rule in &world.rules {
        query::validate(world, rule)?;
        for component in rule.adds.iter().chain(&rule.removes) {
            if !query::is_component(world, component) {
                return Err(syn::Error::new_spanned(
                    component,
                    format!(
                        "unknown component `{component}` in rule `{}` structural effects",
                        rule.name
                    ),
                ));
            }
        }
        dependencies::validate_rule(world, rule)?;
    }
    dependencies::validate_cycles(world)
}
