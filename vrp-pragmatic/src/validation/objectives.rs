#[cfg(test)]
#[path = "../../tests/unit/validation/objectives_test.rs"]
mod objectives_test;

use super::*;
use crate::json::problem::Objective::*;
use std::collections::HashMap;

/// Checks that objective is not empty when specified.
fn check_e1009_empty_objective(objectives: &Vec<&Objective>) -> Result<(), String> {
    if objectives.is_empty() {
        Err("E1009: An empty objective specified".to_string())
    } else {
        Ok(())
    }
}

/// Checks that each objective type specified only once.
fn check_e1010_duplicate_objectives(objectives: &Vec<&Objective>) -> Result<(), String> {
    let mut duplicates = objectives
        .iter()
        .fold(HashMap::new(), |mut acc, objective| {
            match objective {
                MinimizeCost { goal: _ } => acc.entry("minimize-cost"),
                MinimizeTours { goal: _ } => acc.entry("minimize-tours"),
                MinimizeUnassignedJobs { goal: _ } => acc.entry("minimize-unassigned"),
                BalanceMaxLoad { threshold: _, variance: _ } => acc.entry("balance-max-load"),
                BalanceActivities { threshold: _, variance: _ } => acc.entry("balance-activities"),
                BalanceDistance { threshold: _, variance: _ } => acc.entry("balance-distance"),
                BalanceDuration { threshold: _, variance: _ } => acc.entry("balance-duration"),
            }
            .and_modify(|count| *count += 1)
            .or_insert(1_usize);

            acc
        })
        .iter()
        .filter_map(|(name, count)| if *count > 1 { Some(name.to_string()) } else { None })
        .collect::<Vec<_>>();

    duplicates.sort();

    if duplicates.is_empty() {
        Ok(())
    } else {
        Err(format!("E1010: Duplicate objective specified: {}", duplicates.join(",")))
    }
}

/// Checks that cost objective is specified.
fn check_e1011_no_cost_value_objective(objectives: &Vec<&Objective>) -> Result<(), String> {
    let min_costs = objectives
        .iter()
        .filter(|objective| match objective {
            MinimizeCost { goal: _ } => true,
            _ => false,
        })
        .count();

    if min_costs == 0 {
        Err("E1011: Missing cost objective".to_string())
    } else {
        Ok(())
    }
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
