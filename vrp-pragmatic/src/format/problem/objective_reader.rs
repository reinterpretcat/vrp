use crate::core::models::common::ValueDimension;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::BalanceOptions;
use crate::format::problem::Objective::*;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::{ObjectiveCost, TargetConstraint, TargetObjective};
use vrp_core::solver::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> Arc<ObjectiveCost> {
    Arc::new(match (&api_problem.objectives, props.max_job_value) {
        (Some(objectives), _) => ObjectiveCost::new(
            objectives
                .iter()
                .map(|objectives| {
                    let mut core_objectives: Vec<TargetObjective> = vec![];
                    objectives.iter().for_each(|objective| match objective {
                        MinimizeCost => core_objectives.push(TotalCost::minimize()),
                        MinimizeDistance => core_objectives.push(TotalDistance::minimize()),
                        MinimizeDuration => core_objectives.push(TotalDuration::minimize()),
                        MinimizeTours => {
                            constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                            core_objectives.push(Box::new(TotalRoutes::new_minimized()))
                        }
                        MaximizeTours => {
                            constraint.add_module(Box::new(FleetUsageConstraintModule::new_maximized()));
                            core_objectives.push(Box::new(TotalRoutes::new_maximized()))
                        }
                        MaximizeValue { reduction_factor } => {
                            let (module, objective) = TotalValue::maximize(
                                props
                                    .max_job_value
                                    .expect("expecting non-zero job value to be defined at least at on job"),
                                reduction_factor.unwrap_or(0.1),
                                Arc::new(|job| job.dimens().get_value::<f64>("value").cloned().unwrap_or(0.)),
                            );
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        MinimizeUnassignedJobs { breaks } => {
                            if let Some(breaks) = *breaks {
                                core_objectives.push(Box::new(TotalUnassignedJobs::new(Arc::new(move |_, job, _| {
                                    job.dimens().get_value::<String>("type").map_or(1., |job_type| {
                                        if job_type == "break" {
                                            breaks
                                        } else {
                                            1.
                                        }
                                    })
                                }))))
                            } else {
                                core_objectives.push(Box::new(TotalUnassignedJobs::default()))
                            }
                        }
                        BalanceMaxLoad { options } => {
                            let (module, objective) = get_load_balance(props, options);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceActivities { options } => {
                            let (threshold, tolerance) = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_activity_balanced(threshold, tolerance);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceDistance { options } => {
                            let (threshold, tolerance) = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_distance_balanced(threshold, tolerance);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceDuration { options } => {
                            let (threshold, tolerance) = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_duration_balanced(threshold, tolerance);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                    });
                    core_objectives
                })
                .collect(),
        ),
        (_, Some(max_value)) => {
            let (module, objective) = TotalValue::maximize(
                max_value,
                0.1,
                Arc::new(|job| job.dimens().get_value::<f64>("value").cloned().unwrap_or(0.)),
            );

            constraint.add_module(module);
            constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));

            ObjectiveCost::new(vec![
                vec![Box::new(TotalUnassignedJobs::default())],
                vec![Box::new(TotalRoutes::default())],
                vec![objective],
                vec![TotalCost::minimize()],
            ])
        }
        _ => {
            constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
            ObjectiveCost::default()
        }
    })
}

fn unwrap_options(options: &Option<BalanceOptions>) -> (Option<f64>, Option<f64>) {
    (options.as_ref().and_then(|o| o.threshold), options.as_ref().and_then(|o| o.tolerance))
}

fn get_load_balance(
    props: &ProblemProperties,
    options: &Option<BalanceOptions>,
) -> (TargetConstraint, TargetObjective) {
    let (threshold, tolerance) = unwrap_options(options);
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimLoad>(
            threshold,
            tolerance,
            Arc::new(|loaded, total| {
                let mut max_ratio = 0_f64;

                for (idx, value) in total.load.iter().enumerate() {
                    let ratio = loaded.load[idx] as f64 / *value as f64;
                    max_ratio = max_ratio.max(ratio);
                }

                max_ratio
            }),
        )
    } else {
        WorkBalance::new_load_balanced::<SingleDimLoad>(
            threshold,
            tolerance,
            Arc::new(|loaded, capacity| loaded.value as f64 / capacity.value as f64),
        )
    }
}
