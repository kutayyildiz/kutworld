use syn::Result;

use crate::model::{Rule, World};

pub(super) fn validate_rule(world: &World, rule: &Rule) -> Result<()> {
    for dependency in &rule.depends {
        if dependency == &rule.name {
            return Err(syn::Error::new_spanned(
                dependency,
                format!("rule `{}` cannot depend on itself", rule.name),
            ));
        }
        if !world
            .rules
            .iter()
            .any(|candidate| candidate.name == *dependency)
        {
            return Err(syn::Error::new_spanned(
                dependency,
                format!("unknown rule dependency `{dependency}`"),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_cycles(world: &World) -> Result<()> {
    fn visit(index: usize, world: &World, states: &mut [u8]) -> Result<()> {
        states[index] = 1;
        for dependency in &world.rules[index].depends {
            let dependency_index = world
                .rules
                .iter()
                .position(|rule| rule.name == *dependency)
                .expect("rule dependencies were validated");
            if states[dependency_index] == 1 {
                return Err(syn::Error::new_spanned(
                    dependency,
                    format!("dependency cycle includes rule `{dependency}`"),
                ));
            }
            if states[dependency_index] == 0 {
                visit(dependency_index, world, states)?;
            }
        }
        states[index] = 2;
        Ok(())
    }

    let mut states = vec![0; world.rules.len()];
    for index in 0..world.rules.len() {
        if states[index] == 0 {
            visit(index, world, &mut states)?;
        }
    }
    Ok(())
}
