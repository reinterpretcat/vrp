#[cfg(test)]
#[path = "../../../tests/unit/format/problem/objective_reader_test.rs"]
mod objective_reader_test;

use crate::core::models::common::ValueDimension;
use crate::core::models::problem::Job;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::BalanceOptions;
use crate::format::problem::Objective::TourOrder as FormatTourOrder;
use crate::format::problem::Objective::*;
use crate::format::TOUR_ORDER_CONSTRAINT_CODE;
use std::sync::Arc;
use vrp_core::construction::clustering::vicinity::ClusterDimension;
use vrp_core::construction::constraints::{ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::{ObjectiveCost, Single, TargetConstraint, TargetObjective};
use vrp_core::solver::objectives::TourOrder as CoreTourOrder;
use vrp_core::solver::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> Arc<ObjectiveCost> {
    Arc::new(match (&api_problem.objectives, props.max_job_value, props.has_order) {
        (Some(objectives), _, _) => ObjectiveCost::new(
            objectives
                .iter()
                .map(|objectives| {
                    let mut core_objectives: Vec<TargetObjective> = vec![];
                    objectives.iter().for_each(|objective| match objective {
                        MinimizeCost => core_objectives.push(TotalCost::minimize()),
                        MinimizeDistance => core_objectives.push(TotalDistance::minimize()),
                        MinimizeDuration => core_objectives.push(TotalDuration::minimize()),
                        MinimizeTours => {
                            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));
                            core_objectives.push(Arc::new(TotalRoutes::new_minimized()))
                        }
                        MaximizeTours => {
                            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_maximized()));
                            core_objectives.push(Arc::new(TotalRoutes::new_maximized()))
                        }
                        MaximizeValue { breaks, reduction_factor } => {
                            let max_value = props
                                .max_job_value
                                .expect("expecting non-zero job value to be defined at least at on job");
                            let (module, objective) = get_value(max_value, *reduction_factor, *breaks);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        MinimizeUnassignedJobs { breaks } => {
                            if let Some(breaks) = *breaks {
                                core_objectives.push(Arc::new(get_unassigned_objective(breaks)))
                            } else {
                                core_objectives.push(Arc::new(get_unassigned_objective(1.)))
                            }
                        }
                        BalanceMaxLoad { options } => {
                            let (module, objective) = get_load_balance(props, options);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceActivities { options } => {
                            let threshold = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_activity_balanced(threshold);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceDistance { options } => {
                            let threshold = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_distance_balanced(threshold);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        BalanceDuration { options } => {
                            let threshold = unwrap_options(options);
                            let (module, objective) = WorkBalance::new_duration_balanced(threshold);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                        FormatTourOrder { is_constrained } => {
                            let (module, objective) = get_order(*is_constrained);
                            constraint.add_module(module);
                            core_objectives.push(objective);
                        }
                    });
                    core_objectives
                })
                .collect(),
        ),
        (None, Some(max_value), has_order) => {
            let (value_module, value_objective) = get_value(max_value, None, None);

            constraint.add_module(value_module);
            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));

            let mut objectives =
                std::iter::once(vec![value_objective]).chain(get_default_objectives().into_iter()).collect::<Vec<_>>();

            if has_order {
                let (order_module, order_objective) = get_order(false);
                constraint.add_module(order_module);
                objectives.insert(2, vec![order_objective]);
            }

            ObjectiveCost::new(objectives)
        }
        (None, None, true) => {
            let (order_module, order_objective) = get_order(false);

            constraint.add_module(order_module);
            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));

            let mut objectives = get_default_objectives();
            objectives.insert(1, vec![order_objective]);

            ObjectiveCost::new(objectives)
        }
        _ => {
            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));
            ObjectiveCost::new(get_default_objectives())
        }
    })
}

fn unwrap_options(options: &Option<BalanceOptions>) -> Option<f64> {
    options.as_ref().and_then(|o| o.threshold)
}

fn get_value(
    max_value: f64,
    reduction_factor: Option<f64>,
    breaks: Option<f64>,
) -> (TargetConstraint, TargetObjective) {
    // NOTE make it negative
    let break_value = -1. * breaks.unwrap_or(100.);
    TotalValue::maximize(
        max_value,
        reduction_factor.unwrap_or(0.1),
        Arc::new(move |solution| {
            solution.unassigned.iter().map(|(job, _)| get_unassigned_job_estimate(job, break_value, 0.)).sum()
        }),
        Arc::new(|job| job.dimens().get_value::<f64>("value").cloned().unwrap_or(0.)),
        Arc::new(|job, value| match job {
            Job::Single(single) => {
                let mut dimens = single.dimens.clone();
                dimens.set_value("value", value);

                Job::Single(Arc::new(Single { places: single.places.clone(), dimens }))
            }
            _ => job.clone(),
        }),
    )
}

fn get_order(is_constrained: bool) -> (TargetConstraint, TargetObjective) {
    let order_func = Arc::new(|single: &Single| single.dimens.get_value::<i32>("order").map(|order| *order as f64));

    if is_constrained {
        CoreTourOrder::new_constrained(order_func, TOUR_ORDER_CONSTRAINT_CODE)
    } else {
        CoreTourOrder::new_unconstrained(order_func)
    }
}

fn get_load_balance(
    props: &ProblemProperties,
    options: &Option<BalanceOptions>,
) -> (TargetConstraint, TargetObjective) {
    let threshold = unwrap_options(options);
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimLoad>(
            threshold,
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
            Arc::new(|loaded, capacity| loaded.value as f64 / capacity.value as f64),
        )
    }
}

fn get_default_objectives() -> Vec<Vec<TargetObjective>> {
    vec![
        vec![Arc::new(get_unassigned_objective(1.))],
        vec![Arc::new(TotalRoutes::default())],
        vec![TotalCost::minimize()],
    ]
}

fn get_unassigned_objective(break_value: f64) -> TotalUnassignedJobs {
    TotalUnassignedJobs::new(Arc::new(move |_, job, _| get_unassigned_job_estimate(job, break_value, 1.)))
}

fn get_unassigned_job_estimate(job: &Job, break_value: f64, default_value: f64) -> f64 {
    if let Some(clusters) = job.dimens().get_cluster() {
        clusters.len() as f64 * default_value
    } else {
        job.dimens().get_value::<String>("type").map_or(default_value, |job_type| {
            if job_type == "break" {
                break_value
            } else {
                default_value
            }
        })
    }
}
