use crate::extensions::MultiDimensionalCapacity;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::Objective::*;
use crate::format::problem::*;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::models::problem::{ObjectiveCost, TargetConstraint, TargetObjective};
use vrp_core::solver::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> Arc<ObjectiveCost> {
    Arc::new(if let Some(objectives) = &api_problem.objectives {
        let mut map_objectives = |objectives: &Vec<_>| {
            let mut core_objectives: Vec<TargetObjective> = vec![];
            objectives.iter().for_each(|objective| match objective {
                MinimizeCost { goal, tolerance } => {
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalTransportCost::new(
                        value_goal,
                        variation_goal,
                        tolerance.clone(),
                    )))
                }
                MinimizeTours { goal } => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalRoutes::new_minimized(value_goal, variation_goal)))
                }
                MaximizeTours => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_maximized()));
                    core_objectives.push(Box::new(TotalRoutes::new_maximized()))
                }
                MinimizeUnassignedJobs { goal } => {
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalUnassignedJobs::new(value_goal, variation_goal)))
                }
                BalanceMaxLoad { threshold, tolerance } => {
                    let (module, objective) = get_load_balance(props, threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceActivities { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_activity_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDistance { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_distance_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDuration { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_duration_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
            });
            core_objectives
        };

        let primary_objectives = map_objectives(&objectives.primary);
        let secondary_objectives = map_objectives(&objectives.secondary.clone().unwrap_or_else(|| vec![]));

        ObjectiveCost::new(primary_objectives, secondary_objectives)
    } else {
        constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
        ObjectiveCost::default()
    })
}

fn get_load_balance(
    props: &ProblemProperties,
    threshold: Option<f64>,
    tolerance: Option<BalanceTolerance>,
) -> (TargetConstraint, TargetObjective) {
    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance);
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimensionalCapacity>(
            threshold,
            solution_tolerance,
            route_tolerance,
            Arc::new(|loaded, total| {
                let mut max_ratio = 0_f64;

                for (idx, value) in total.capacity.iter().enumerate() {
                    let ratio = loaded.capacity[idx] as f64 / *value as f64;
                    max_ratio = max_ratio.max(ratio);
                }

                max_ratio
            }),
        )
    } else {
        WorkBalance::new_load_balanced::<i32>(
            threshold,
            solution_tolerance,
            route_tolerance,
            Arc::new(|loaded, capacity| *loaded as f64 / *capacity as f64),
        )
    }
}

fn get_balance_tolerance_params(tolerance: Option<BalanceTolerance>) -> (Option<f64>, Option<f64>) {
    if let Some(tolerance) = tolerance {
        (tolerance.solution, tolerance.route)
    } else {
        (None, None)
    }
}

fn split_goal<T: Clone>(goal: &Option<GoalSatisfactionCriteria<T>>) -> (Option<T>, Option<(usize, f64)>) {
    goal.as_ref()
        .map_or((None, None), |goal| (goal.value.clone(), goal.variation.as_ref().map(|vc| (vc.sample, vc.variation))))
}
