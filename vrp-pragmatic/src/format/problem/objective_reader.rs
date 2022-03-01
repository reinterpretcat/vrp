#[cfg(test)]
#[path = "../../../tests/unit/format/problem/objective_reader_test.rs"]
mod objective_reader_test;

use crate::constraints::{AreaModule, TOTAL_VALUE_KEY, TOUR_ORDER_KEY};
use crate::core::models::common::IdDimension;
use crate::format::problem::reader::{ApiProblem, ProblemProperties};
use crate::format::problem::BalanceOptions;
use crate::format::problem::Objective::TourOrder as FormatTourOrder;
use crate::format::problem::Objective::*;
use crate::format::{AREA_CONSTRAINT_CODE, TOUR_ORDER_CONSTRAINT_CODE};
use hashbrown::HashMap;
use std::sync::Arc;
use vrp_core::construction::clustering::vicinity::ClusterDimension;
use vrp_core::construction::constraints::{ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::models::common::ValueDimension;
use vrp_core::models::common::{MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::Job;
use vrp_core::models::problem::{ProblemObjective, Single, TargetConstraint, TargetObjective};
use vrp_core::solver::objectives::TourOrder as CoreTourOrder;
use vrp_core::solver::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> Arc<ProblemObjective> {
    Arc::new(match &api_problem.objectives {
        Some(objectives) => ProblemObjective::new(
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
                        AreaOrder { breaks, is_constrained, is_value_preferred } => {
                            let max_value = props.max_area_value.unwrap_or(1.);
                            let (module, objectives) =
                                get_area(max_value, *breaks, *is_constrained, is_value_preferred.unwrap_or(false));

                            constraint.add_module(module);
                            objectives.into_iter().for_each(|objective| core_objectives.push(objective));
                        }
                    });
                    core_objectives
                })
                .collect(),
        ),
        None => {
            let mut objectives: Vec<Vec<TargetObjective>> = vec![
                vec![Arc::new(get_unassigned_objective(1.))],
                vec![Arc::new(TotalRoutes::default())],
                vec![TotalCost::minimize()],
            ];
            constraint.add_module(Arc::new(FleetUsageConstraintModule::new_minimized()));

            if let Some(max_value) = props.max_job_value {
                let (value_module, value_objective) = get_value(max_value, None, None);
                objectives.insert(0, vec![value_objective]);
                constraint.add_module(value_module);
            }

            if props.has_order {
                let (order_module, order_objective) = get_order(false);
                constraint.add_module(order_module);
                objectives.insert(if props.max_job_value.is_some() { 2 } else { 1 }, vec![order_objective]);
            }

            ProblemObjective::new(objectives)
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
        ValueFn::Left(Arc::new(|job| job.dimens().get_value::<f64>("value").cloned().unwrap_or(0.))),
        Arc::new(|job, value| match job {
            Job::Single(single) => {
                let mut dimens = single.dimens.clone();
                dimens.set_value("value", value);

                Job::Single(Arc::new(Single { places: single.places.clone(), dimens }))
            }
            _ => job.clone(),
        }),
        TOTAL_VALUE_KEY,
        -1,
    )
}

fn get_order(is_constrained: bool) -> (TargetConstraint, TargetObjective) {
    let order_fn = OrderFn::Left(Arc::new(|single| single.dimens.get_value::<i32>("order").map(|order| *order as f64)));

    if is_constrained {
        CoreTourOrder::new_constrained(order_fn, TOUR_ORDER_KEY, TOUR_ORDER_CONSTRAINT_CODE)
    } else {
        CoreTourOrder::new_unconstrained(order_fn, TOUR_ORDER_KEY)
    }
}

fn get_area(
    max_value: f64,
    break_value: Option<f64>,
    is_constrained: bool,
    is_value_preferred: bool,
) -> (TargetConstraint, Vec<TargetObjective>) {
    let break_value = break_value.unwrap_or(100.);

    let order_fn: ActorOrderFn = Arc::new(|actor, single| {
        actor
            .vehicle
            .dimens
            .get_value::<HashMap<String, (usize, f64)>>("areas")
            .and_then(|index| single.dimens.get_id().and_then(|id| index.get(id)))
            .map(|(order, _)| *order as f64)
    });
    let value_fn: ActorValueFn = Arc::new(|actor, job| {
        actor
            .vehicle
            .dimens
            .get_value::<HashMap<String, (usize, f64)>>("areas")
            .and_then(|index| job.dimens().get_id().and_then(|id| index.get(id)))
            .map(|(_, value)| *value)
            .unwrap_or(0.)
    });
    let solution_fn: SolutionValueFn = Arc::new(move |solution| {
        solution.unassigned.iter().map(|(job, _)| get_unassigned_job_estimate(job, break_value, 0.)).sum()
    });

    if is_constrained {
        AreaModule::new_constrained(order_fn, value_fn, solution_fn, max_value, AREA_CONSTRAINT_CODE)
    } else {
        AreaModule::new_unconstrained(order_fn, value_fn, solution_fn, max_value, is_value_preferred)
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
