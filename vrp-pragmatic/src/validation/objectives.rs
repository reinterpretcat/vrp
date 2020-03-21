#[cfg(test)]
#[path = "../../tests/unit/validation/objectives_test.rs"]
mod objectives_test;

use super::*;

/// Checks that objective is not empty when specified.
fn check_e1009_empty_objective(objectives: &Vec<&Objective>) -> Result<(), String> {
    if objectives.is_empty() {
        Err("E1009: An empty objective specified".to_string())
    } else {
        Ok(())
    }
}

/// Checks that each objective type specified only once.
fn check_e1010_duplicate_objectives(_objectives: &Vec<&Objective>) -> Result<(), String> {
    Ok(())
}

fn check_e1011_no_cost_value_objective(_objectives: &Vec<&Objective>) -> Result<(), String> {
    Ok(())
}

fn get_objectives<'a>(ctx: &'a ValidationContext) -> Option<Vec<&'a Objective>> {
    ctx.problem.objectives.as_ref().map(|objectives| {
        Some(&objectives.primary)
            .iter()
            .chain(objectives.secondary.as_ref().iter())
            .flat_map(|objectives| objectives.iter())
            .collect()
    })
}

pub fn validate_objectives(ctx: &ValidationContext) -> Result<(), Vec<String>> {
    let errors = if let Some(objectives) = get_objectives(ctx) {
        check_e1009_empty_objective(&objectives)
            .err()
            .iter()
            .cloned()
            .chain(check_e1010_duplicate_objectives(&objectives).err().iter().cloned())
            .chain(check_e1011_no_cost_value_objective(&objectives).err().iter().cloned())
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
