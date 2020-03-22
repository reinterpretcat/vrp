use crate::constraints::WorkBalance;
use crate::extensions::MultiDimensionalCapacity;
use crate::json::problem::reader::{ApiProblem, ProblemProperties};
use crate::json::problem::Objective::*;
use crate::json::problem::*;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintModule, ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::refinement::objectives::Objective as CoreObjective;
use vrp_core::refinement::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> MultiObjective {
    if let Some(objectives) = &api_problem.objectives {
        let mut map_objectives = |objectives: &Vec<_>| {
            let mut core_objectives: Vec<Box<dyn CoreObjective + Send + Sync>> = vec![];
            let mut cost_idx = None;
            objectives.iter().enumerate().for_each(|(idx, objective)| match objective {
                MinimizeCost { goal } => {
                    cost_idx = Some(idx);
                    core_objectives.push(Box::new(match goal {
                        Some(GoalSatisfactionCriteria { value: None, variation: None }) => {
                            TotalTransportCost::default()
                        }
                        Some(GoalSatisfactionCriteria { value, variation }) => TotalTransportCost::new(
                            value.clone(),
                            variation.as_ref().map(|vc| (vc.sample, vc.variation)),
                        ),
                        _ => TotalTransportCost::default(),
                    }));
                }
                MinimizeTours { goal } => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                    core_objectives.push(Box::new(match goal {
                        Some(GoalSatisfactionCriteria { value: None, variation: None }) => TotalRoutes::default(),
                        Some(GoalSatisfactionCriteria { value, variation }) => TotalRoutes::new(
                            value.clone(),
                            variation.as_ref().map(|vc| (vc.sample, vc.variation)),
                            true,
                        ),
                        _ => TotalRoutes::default(),
                    }));
                }
                MinimizeUnassignedJobs { goal } => {
                    core_objectives.push(Box::new(match goal {
                        Some(GoalSatisfactionCriteria { value: None, variation: None }) => {
                            TotalUnassignedJobs::default()
                        }
                        Some(GoalSatisfactionCriteria { value, variation }) => TotalUnassignedJobs::new(
                            value.clone(),
                            variation.as_ref().map(|vc| (vc.sample, vc.variation)),
                        ),
                        _ => TotalUnassignedJobs::default(),
                    }));
                }
                BalanceMaxLoad { threshold, tolerance } => {
                    let (module, objective) = get_load_balance(props, threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceActivities { threshold, tolerance } => {
                    let (module, objective) = WorkBalance::new_activity_balanced(threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDistance { threshold, tolerance } => {
                    let (module, objective) = WorkBalance::new_distance_balanced(threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDuration { threshold, tolerance } => {
                    let (module, objective) = WorkBalance::new_duration_balanced(threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
            });
            (core_objectives, cost_idx)
        };

        let (primary, primary_cost_idx) = map_objectives(&objectives.primary);
        let (secondary, secondary_cost_idx) = map_objectives(&objectives.secondary.clone().unwrap_or_else(|| vec![]));

        MultiObjective::new(
            primary,
            secondary,
            Arc::new(move |primary, secondary| {
                primary_cost_idx
                    .map(|idx| primary.get(idx).unwrap())
                    .or(secondary_cost_idx.map(|idx| secondary.get(idx).unwrap()))
                    .expect("Cannot get cost value objective")
                    .value()
            }),
        )
    } else {
        constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
        MultiObjective::default()
    }
}

fn get_load_balance(
    props: &ProblemProperties,
    threshold: Option<f64>,
    variance: Option<f64>,
) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn CoreObjective + Send + Sync>) {
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimensionalCapacity>(
            threshold,
            variance,
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
            variance,
            Arc::new(|loaded, capacity| *loaded as f64 / *capacity as f64),
        )
    }
}
