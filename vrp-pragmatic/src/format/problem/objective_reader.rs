use crate::extensions::MultiDimensionalCapacity;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::Objective::*;
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
                MinimizeCost => core_objectives.push(Box::new(TotalTransportCost::default())),
                MinimizeTours => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                    core_objectives.push(Box::new(TotalRoutes::new_minimized()))
                }
                MaximizeTours => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_maximized()));
                    core_objectives.push(Box::new(TotalRoutes::new_maximized()))
                }
                MinimizeUnassignedJobs => core_objectives.push(Box::new(TotalUnassignedJobs::default())),
                BalanceMaxLoad { threshold } => {
                    let (module, objective) = get_load_balance(props, threshold.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceActivities { threshold } => {
                    let (module, objective) = WorkBalance::new_activity_balanced(threshold.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDistance { threshold } => {
                    let (module, objective) = WorkBalance::new_distance_balanced(threshold.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDuration { threshold } => {
                    let (module, objective) = WorkBalance::new_duration_balanced(threshold.clone());
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

fn get_load_balance(props: &ProblemProperties, threshold: Option<f64>) -> (TargetConstraint, TargetObjective) {
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimensionalCapacity>(
            threshold,
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
        WorkBalance::new_load_balanced::<i32>(threshold, Arc::new(|loaded, capacity| *loaded as f64 / *capacity as f64))
    }
}
