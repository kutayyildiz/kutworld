use syn::{Item, Result};

use crate::model::{Rule, World};

mod attributes;
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
        if attributes::has_attribute(&function.attrs, "write_only") {
            let attr = attributes::one_attribute(&function.attrs, "write_only")?.unwrap();
            return Err(syn::Error::new_spanned(
                attr,
                "`#[write_only]` is only valid on an `&mut Component` rule parameter",
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
        let accesses = views::collect(&mut function.sig)?;
        let adds = attributes::parse_name_attribute(&function.attrs, "adds")?;
        let removes = attributes::parse_name_attribute(&function.attrs, "removes")?;
        if attributes::has_attribute(&function.attrs, "depends") {
            let attr = attributes::one_attribute(&function.attrs, "depends")?.unwrap();
            return Err(syn::Error::new_spanned(
                attr,
                "`#[depends(...)]` has been removed; dependencies are inferred from rule effects",
            ));
        }
        let break_cycle = attributes::parse_name_attribute(&function.attrs, "break_cycle")?;
        let spawns = attributes::parse_flag_attribute(&function.attrs, "spawns")?;
        let despawns = attributes::parse_flag_attribute(&function.attrs, "despawns")?;
        let serial = attributes::parse_flag_attribute(&function.attrs, "serial")?;
        attributes::remove_rule_attributes(&mut function.attrs);
        world.rules.push(Rule {
            name: function.sig.ident.clone(),
            has,
            not,
            accesses,
            adds,
            removes,
            break_cycle,
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
    }
    let mut names = Vec::new();
    for rule in &world.rules {
        if names.contains(&rule.name) {
            return Err(syn::Error::new_spanned(
                &rule.name,
                format!("duplicate rule name `{}`", rule.name),
            ));
        }
        names.push(rule.name.clone());
    }
    Ok(())
}
