use crate::constraints::WorkBalanceModule;
use crate::extensions::MultiDimensionalCapacity;
use crate::json::problem::reader::{ApiProblem, ProblemProperties};
use crate::json::problem::Objective::*;
use crate::json::problem::*;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintPipeline, FleetUsageConstraintModule};
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
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: Some(_variation) }) => {
                            TotalTransportCost::new(*value)
                        }
                        Some(GoalSatisfactionCriteria { value: None, variation: Some(_variation) }) => {
                            TotalTransportCost::default()
                        }
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: None }) => {
                            TotalTransportCost::new(*value)
                        }
                        _ => TotalTransportCost::default(),
                    }));
                }
                MinimizeTours { goal } => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                    core_objectives.push(Box::new(match goal {
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: Some(_variation) }) => {
                            TotalRoutes::new(*value, true)
                        }
                        Some(GoalSatisfactionCriteria { value: None, variation: Some(_variation) }) => {
                            TotalRoutes::default()
                        }
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: None }) => {
                            TotalRoutes::new(*value, true)
                        }
                        _ => TotalRoutes::default(),
                    }));
                }
                MinimizeUnassignedJobs { goal } => {
                    core_objectives.push(Box::new(match goal {
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: Some(_variation) }) => {
                            TotalUnassignedJobs::new(*value)
                        }
                        Some(GoalSatisfactionCriteria { value: None, variation: Some(_variation) }) => {
                            TotalUnassignedJobs::default()
                        }
                        Some(GoalSatisfactionCriteria { value: Some(value), variation: None }) => {
                            TotalUnassignedJobs::new(*value)
                        }
                        _ => TotalUnassignedJobs::default(),
                    }));
                }
                BalanceMaxLoad { threshold: _ } => {
                    add_work_balance_module(constraint, props);
                }
                BalanceActivities { threshold: _ } => todo!("Balance activities is not yet implemented"),
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

fn add_work_balance_module(constraint: &mut ConstraintPipeline, props: &ProblemProperties) {
    // TODO do not use hard coded penalty
    let balance_penalty = 1000.;
    if props.has_multi_dimen_capacity {
        constraint.add_module(Box::new(WorkBalanceModule::new_load_balanced::<MultiDimensionalCapacity>(
            balance_penalty,
            Box::new(|loaded, total| {
                let mut max_ratio = 0_f64;

                for (idx, value) in total.capacity.iter().enumerate() {
                    let ratio = loaded.capacity[idx] as f64 / *value as f64;
                    max_ratio = max_ratio.max(ratio);
                }

                max_ratio
            }),
        )));
    } else {
        constraint.add_module(Box::new(WorkBalanceModule::new_load_balanced::<i32>(
            balance_penalty,
            Box::new(|loaded, capacity| *loaded as f64 / *capacity as f64),
        )));
    }
}
