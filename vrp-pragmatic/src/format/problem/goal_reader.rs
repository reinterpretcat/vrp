use super::*;
use std::ops::Mul;
use vrp_core::construction::clustering::vicinity::ClusterInfoDimension;
use vrp_core::construction::enablers::FeatureCombinator;
use vrp_core::construction::features::*;
use vrp_core::models::common::{Demand, LoadOps, MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::{Actor, Single, TransportCost};
use vrp_core::models::solution::Route;
use vrp_core::models::{Feature, FeatureObjective, GoalBuilder, GoalContext, GoalContextBuilder};
use vrp_core::rosomaxa::evolution::objectives::dominance_order;

pub(super) fn create_goal_context(
    api_problem: &ApiProblem,
    blocks: &ProblemBlocks,
    props: &ProblemProperties,
) -> GenericResult<GoalContext> {
    // determine features from objective definition
    let feature_layers = get_objective_feature_layers(api_problem, blocks, props)?;
    let (mut features, goal_builder) = get_features_with_goal(&feature_layers)?;

    if props.has_unreachable_locations {
        features.push(create_reachable_feature("reachable", blocks.transport.clone(), REACHABLE_CONSTRAINT_CODE)?)
    }

    features.push(get_capacity_feature("capacity", api_problem, blocks, props)?);

    if props.has_tour_travel_limits {
        features.push(get_tour_limit_feature("tour_limit", api_problem, blocks.transport.clone())?)
    }

    if props.has_breaks {
        features.push(create_optional_break_feature("break")?)
    }

    if props.has_recharges {
        features.push(get_recharge_feature("recharge", api_problem, blocks.transport.clone())?);
    }

    if props.has_order && !features.iter().any(|f| f.name == "tour_order") {
        features.push(create_tour_order_hard_feature("tour_order", TOUR_ORDER_CONSTRAINT_CODE, get_tour_order_fn())?)
    }

    if props.has_compatibility {
        features.push(create_compatibility_feature("compatibility", COMPATIBILITY_CONSTRAINT_CODE)?);
    }

    if props.has_group {
        features.push(create_group_feature("group", blocks.jobs.size(), GROUP_CONSTRAINT_CODE)?);
    }

    if props.has_skills {
        features.push(create_skills_feature("skills", SKILL_CONSTRAINT_CODE)?)
    }

    if !blocks.locks.is_empty() {
        features.push(create_locked_jobs_feature(
            "locked_jobs",
            blocks.fleet.as_ref(),
            &blocks.locks,
            LOCKING_CONSTRAINT_CODE,
        )?);
    }

    if props.has_tour_size_limits {
        features.push(create_activity_limit_feature(
            "activity_limit",
            TOUR_SIZE_CONSTRAINT_CODE,
            Arc::new(|actor| actor.vehicle.dimens.get_tour_size().copied()),
        )?);
    }

    GoalContextBuilder::with_features(&features)?.set_main_goal(goal_builder.build()?).build()
}

/// Layer retains information about whether a feature is defined as standalone or as having some competitive.
enum FeatureLayer {
    /// A single feature layer: no competitive objectives.
    Single(Feature),

    /// A multi feature layer: multiple competitive objectives are available for multiple features.
    Multi { composition_type: CompositionType, features: Vec<Feature> },
}

fn get_objective_feature_layers(
    api_problem: &ApiProblem,
    blocks: &ProblemBlocks,
    props: &ProblemProperties,
) -> GenericResult<Vec<FeatureLayer>> {
    let objectives = get_objectives(api_problem, props);

    objectives
        .iter()
        .map(|objective| get_objective_feature_layer(objective, blocks, props))
        .collect::<GenericResult<_>>()
}

fn get_objective_feature_layer(
    objective: &Objective,
    blocks: &ProblemBlocks,
    props: &ProblemProperties,
) -> GenericResult<FeatureLayer> {
    let feature = match objective {
        Objective::MinimizeCost => TransportFeatureBuilder::new("min_cost")
            .set_violation_code(TIME_CONSTRAINT_CODE)
            .set_transport_cost(blocks.transport.clone())
            .set_activity_cost(blocks.activity.clone())
            .build_minimize_cost(),
        Objective::MinimizeDistance => TransportFeatureBuilder::new("min_distance")
            .set_violation_code(TIME_CONSTRAINT_CODE)
            .set_transport_cost(blocks.transport.clone())
            .set_activity_cost(blocks.activity.clone())
            .build_minimize_distance(),
        Objective::MinimizeDuration => TransportFeatureBuilder::new("min_duration")
            .set_violation_code(TIME_CONSTRAINT_CODE)
            .set_transport_cost(blocks.transport.clone())
            .set_activity_cost(blocks.activity.clone())
            .build_minimize_duration(),
        Objective::MinimizeTours => create_minimize_tours_feature("min_tours"),
        Objective::MaximizeTours => create_maximize_tours_feature("max_tours"),
        Objective::MaximizeValue { breaks } => create_maximize_total_job_value_feature(
            "max_value",
            JobReadValueFn::Left(Arc::new({
                let break_value = *breaks;
                move |job| {
                    job.dimens().get_job_value().copied().unwrap_or_else(|| {
                        job.dimens()
                            .get_job_type()
                            .zip(break_value)
                            .filter(|(job_type, _)| *job_type == "break")
                            .map(|(_, break_value)| break_value)
                            .unwrap_or(0.)
                    })
                }
            })),
            Arc::new(|job, value| match job {
                CoreJob::Single(single) => {
                    let mut dimens = single.dimens.clone();
                    dimens.set_job_value(value);

                    CoreJob::Single(Arc::new(Single { places: single.places.clone(), dimens }))
                }
                _ => job.clone(),
            }),
            -1,
        ),
        Objective::MinimizeUnassigned { breaks } => MinimizeUnassignedBuilder::new("min_unassigned")
            .set_job_estimator({
                let break_value = *breaks;
                let default_value = 1.;
                move |_, job| {
                    if let Some(clusters) = job.dimens().get_cluster_info() {
                        clusters.len() as f64 * default_value
                    } else {
                        job.dimens().get_job_type().map_or(default_value, |job_type| match job_type.as_str() {
                            "break" => break_value.unwrap_or(default_value),
                            _ => default_value,
                        })
                    }
                }
            })
            .build(),

        Objective::MinimizeArrivalTime => create_minimize_arrival_time_feature("min_arrival_time"),
        Objective::BalanceMaxLoad => {
            if props.has_multi_dimen_capacity {
                create_max_load_balanced_feature::<MultiDimLoad>(
                    "max_load_balance",
                    |loaded, capacity| {
                        let mut max_ratio = 0_f64;

                        for (idx, value) in capacity.load.iter().enumerate() {
                            let ratio = loaded.load[idx] as f64 / *value as f64;
                            max_ratio = max_ratio.max(ratio);
                        }

                        max_ratio
                    },
                    |vehicle| vehicle.dimens.get_vehicle_capacity().expect("vehicle has no capacity defined"),
                )
            } else {
                create_max_load_balanced_feature::<SingleDimLoad>(
                    "max_load_balance",
                    |loaded, capacity| loaded.value as f64 / capacity.value as f64,
                    |vehicle| vehicle.dimens.get_vehicle_capacity().expect("vehicle has no capacity defined"),
                )
            }
        }
        Objective::BalanceActivities => create_activity_balanced_feature("activity_balance"),
        Objective::BalanceDistance => create_distance_balanced_feature("distance_balance"),
        Objective::BalanceDuration => create_duration_balanced_feature("duration_balance"),
        Objective::CompactTour { job_radius } => {
            create_tour_compactness_feature("tour_compact", blocks.jobs.clone(), *job_radius)
        }
        Objective::TourOrder => create_tour_order_soft_feature("tour_order", get_tour_order_fn()),
        Objective::FastService => get_fast_service_feature("fast_service", blocks),
        Objective::Composite { objectives, composition_type } => {
            let features = objectives
                .iter()
                .map(|o| get_objective_feature_layer(o, blocks, props))
                .map(|layer| match layer {
                    Ok(FeatureLayer::Single(feature)) => Ok(feature),
                    Ok(FeatureLayer::Multi { .. }) => {
                        Err(GenericError::from("nested composite objectives are not supported"))
                    }
                    Err(err) => Err(err),
                })
                .collect::<GenericResult<Vec<_>>>()?;
            let composition_type = composition_type.clone();

            return Ok(FeatureLayer::Multi { features, composition_type });
        }
    }?;

    Ok(FeatureLayer::Single(feature))
}

fn get_objectives(api_problem: &ApiProblem, props: &ProblemProperties) -> Vec<Objective> {
    if let Some(objectives) = api_problem.objectives.clone() {
        objectives
    } else {
        let mut objectives =
            vec![Objective::MinimizeUnassigned { breaks: Some(1.) }, Objective::MinimizeTours, Objective::MinimizeCost];

        if props.has_value {
            objectives.insert(0, Objective::MaximizeValue { breaks: None })
        }

        objectives
    }
}

fn get_features_with_goal(feature_layers: &[FeatureLayer]) -> GenericResult<(Vec<Feature>, GoalBuilder)> {
    feature_layers.iter().try_fold((Vec::default(), GoalBuilder::default()), |(mut all_features, builder), layer| {
        Ok(match layer {
            FeatureLayer::Single(feature) => {
                let objective = feature
                    .objective
                    .clone()
                    .ok_or_else(|| format!("feature '{}' has no objective while used as objective", feature.name))?;

                all_features.push(feature.clone());

                (all_features, builder.add_single(objective))
            }
            FeatureLayer::Multi { composition_type, features } => {
                let objectives = features
                    .iter()
                    .map(|f| {
                        f.objective.clone().ok_or_else(|| {
                            format!("feature '{}' has no objective while used as objective", f.name).into()
                        })
                    })
                    .collect::<GenericResult<Vec<_>>>()?;

                all_features.push(
                    FeatureCombinator::default()
                        .add_features(features)
                        // objectives are combined using [eval_objective_composition_type] function
                        .set_objective_combinator(|_| None)
                        .combine()?,
                );

                (all_features, eval_objective_composition_type(&objectives, composition_type, builder)?)
            }
        })
    })
}

fn eval_objective_composition_type(
    objectives: &[Arc<dyn FeatureObjective>],
    composition_type: &CompositionType,
    builder: GoalBuilder,
) -> GenericResult<GoalBuilder> {
    Ok(match composition_type {
        CompositionType::Sum => builder.add_multi(
            objectives,
            |os, a, b| dominance_order(a, b, os.iter().map(|o| |a, b| compare_floats(o.fitness(a), o.fitness(b)))),
            |os, move_ctx| os.iter().map(|o| o.estimate(move_ctx)).sum(),
        ),

        CompositionType::WeightedSum { weights } => {
            if objectives.len() != weights.len() {
                return Err(format!(
                    "weighted sum requires same amount of weights as objective count: {} vs {}",
                    weights.len(),
                    objectives.len()
                )
                .into());
            }

            builder.add_multi(
                objectives,
                |os, a, b| dominance_order(a, b, os.iter().map(|o| |a, b| compare_floats(o.fitness(a), o.fitness(b)))),
                {
                    let weights = weights.clone();
                    move |os, move_ctx| os.iter().enumerate().map(|(idx, o)| o.estimate(move_ctx) * weights[idx]).sum()
                },
            )
        }
    })
}

fn get_capacity_feature(
    name: &str,
    api_problem: &ApiProblem,
    blocks: &ProblemBlocks,
    props: &ProblemProperties,
) -> Result<Feature, GenericError> {
    // NOTE: reload uses capacity feature implicitly
    if props.has_reloads {
        if props.has_multi_dimen_capacity {
            create_capacity_with_reload_feature::<MultiDimLoad>(name, api_problem, blocks, MultiDimLoad::new)
        } else {
            create_capacity_with_reload_feature::<SingleDimLoad>(name, api_problem, blocks, |capacity| {
                SingleDimLoad::new(capacity.first().cloned().unwrap_or_default())
            })
        }
    } else if props.has_multi_dimen_capacity {
        CapacityFeatureBuilder::<MultiDimLoad>::new(name).set_violation_code(CAPACITY_CONSTRAINT_CODE).build()
    } else {
        CapacityFeatureBuilder::<SingleDimLoad>::new(name).set_violation_code(CAPACITY_CONSTRAINT_CODE).build()
    }
}

fn get_fast_service_feature(name: &str, blocks: &ProblemBlocks) -> GenericResult<Feature> {
    let (transport, activity) = (blocks.transport.clone(), blocks.activity.clone());

    FastServiceFeatureBuilder::new(name)
        .set_transport(transport)
        .set_activity(activity)
        .set_demand_type_fn(|single| {
            let demand_single: Option<&Demand<SingleDimLoad>> = single.dimens.get_job_demand();
            let demand_multi: Option<&Demand<MultiDimLoad>> = single.dimens.get_job_demand();

            demand_single.map(|d| d.get_type()).or_else(|| demand_multi.map(|d| d.get_type()))
        })
        .set_is_filtered_job(|job| job.dimens().get_job_type().map_or(false, |job_type| job_type == "reload"))
        .build()
}

fn create_capacity_with_reload_feature<T: LoadOps + SharedResource + Mul<f64, Output = T>>(
    name: &str,
    api_problem: &ApiProblem,
    blocks: &ProblemBlocks,
    capacity_map: fn(Vec<i32>) -> T,
) -> GenericResult<Feature> {
    const RELOAD_THRESHOLD: f64 = 0.9;

    fn is_reload_single(single: &Single) -> bool {
        single.dimens.get_job_type().map_or(false, |job_type| job_type == "reload")
    }

    let builder = ReloadFeatureFactory::new(name)
        .set_capacity_code(CAPACITY_CONSTRAINT_CODE)
        .set_load_schedule_threshold(move |capacity: &T| *capacity * RELOAD_THRESHOLD)
        .set_is_reload_single(is_reload_single)
        .set_belongs_to_route(|route: &Route, job: &CoreJob| {
            job.as_single()
                .map_or(false, |single| is_reload_single(single.as_ref()) && is_correct_vehicle(route, single))
        });

    let job_index = blocks.job_index.as_ref().ok_or("misconfiguration in goal reader: job index is not set")?;
    let reload_resources = get_reload_resources(api_problem, job_index, capacity_map);

    if reload_resources.is_empty() {
        builder.build_simple()
    } else {
        let total_jobs = blocks.jobs.size();
        builder
            .set_resource_code(RELOAD_RESOURCE_CONSTRAINT_CODE)
            .set_shared_demand_capacity(|single| single.dimens.get_job_demand().map(|demand| demand.delivery.0))
            .set_shared_resource_capacity(move |activity| {
                activity
                    .job
                    .as_ref()
                    .filter(|single| is_reload_single(single.as_ref()))
                    .and_then(|single| reload_resources.get(&CoreJob::Single(single.clone())).cloned())
            })
            .set_load_schedule_threshold(move |capacity: &T| *capacity * RELOAD_THRESHOLD)
            .set_is_partial_solution(move |solution_ctx| solution_ctx.get_jobs_amount() != total_jobs)
            .build_shared()
    }
}

fn get_tour_limit_feature(
    name: &str,
    api_problem: &ApiProblem,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> GenericResult<Feature> {
    let (distances, durations) = api_problem
        .fleet
        .vehicles
        .iter()
        .filter_map(|vehicle| vehicle.limits.as_ref().map(|limits| (vehicle, limits)))
        .fold((HashMap::new(), HashMap::new()), |(mut distances, mut durations), (vehicle, limits)| {
            limits.max_distance.iter().for_each(|max_distance| {
                distances.insert(vehicle.type_id.clone(), *max_distance);
            });

            limits.max_duration.iter().for_each(|max_duration| {
                durations.insert(vehicle.type_id.clone(), *max_duration);
            });

            (distances, durations)
        });

    let get_limit = |limit_map: HashMap<String, f64>| {
        Arc::new(move |actor: &Actor| {
            actor.vehicle.dimens.get_vehicle_type().and_then(|v_type| limit_map.get(v_type)).cloned()
        })
    };

    create_travel_limit_feature(
        name,
        transport.clone(),
        DISTANCE_LIMIT_CONSTRAINT_CODE,
        DURATION_LIMIT_CONSTRAINT_CODE,
        get_limit(distances),
        get_limit(durations),
    )
}

fn get_recharge_feature(
    name: &str,
    api_problem: &ApiProblem,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> GenericResult<Feature> {
    fn is_recharge_single(single: &Single) -> bool {
        single.dimens.get_job_type().map_or(false, |job_type| job_type == "recharge")
    }

    let distance_limit_index: HashMap<_, HashMap<_, _>> =
        api_problem.fleet.vehicles.iter().fold(HashMap::default(), |mut acc, vehicle_type| {
            vehicle_type
                .shifts
                .iter()
                .enumerate()
                .flat_map(|(shift_idx, shift)| {
                    shift.recharges.as_ref().map(|recharges| (shift_idx, recharges.max_distance))
                })
                .for_each(|(shift_idx, max_distance)| {
                    acc.entry(vehicle_type.type_id.clone()).or_default().insert(shift_idx, max_distance);
                });

            acc
        });

    RechargeFeatureBuilder::new(name)
        .set_violation_code(RECHARGE_CONSTRAINT_CODE)
        .set_transport(transport)
        .set_is_recharge_single(is_recharge_single)
        .set_belongs_to_route(|route, job| {
            job.as_single()
                .map_or(false, |single| is_recharge_single(single.as_ref()) && is_correct_vehicle(route, single))
        })
        .set_distance_limit(move |actor| {
            actor.vehicle.dimens.get_vehicle_type().zip(actor.vehicle.dimens.get_shift_index().copied()).and_then(
                |(type_id, shift_idx)| distance_limit_index.get(type_id).and_then(|idx| idx.get(&shift_idx).copied()),
            )
        })
        .build()
}

fn get_reload_resources<T>(
    api_problem: &ApiProblem,
    job_index: &JobIndex,
    capacity_map: fn(Vec<i32>) -> T,
) -> HashMap<CoreJob, (T, SharedResourceId)>
where
    T: LoadOps + SharedResource,
{
    // get available resources
    let available_resources = api_problem
        .fleet
        .resources
        .as_ref()
        .iter()
        .flat_map(|resources| resources.iter())
        .map(|resource| match resource {
            VehicleResource::Reload { id, capacity } => (id.clone(), capacity.clone()),
        })
        .collect::<Vec<_>>();
    let total_resources_specified = available_resources.len();
    let available_resources = available_resources
        .into_iter()
        .enumerate()
        .map(|(idx, (id, capacity))| (id, (idx, capacity)))
        .collect::<HashMap<_, _>>();
    assert_eq!(total_resources_specified, available_resources.len());

    // get reload resources
    api_problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|vehicle| {
            vehicle
                .shifts
                .iter()
                .enumerate()
                .flat_map(|(shift_idx, vehicle_shift)| {
                    vehicle_shift
                        .reloads
                        .iter()
                        .flatten()
                        .enumerate()
                        .map(move |(reload_idx, reload)| (shift_idx, reload_idx + 1, reload))
                })
                .filter_map(|(shift_idx, place_idx, reload)| {
                    reload
                        .resource_id
                        .as_ref()
                        .and_then(|resource_id| available_resources.get(resource_id))
                        .map(|(resource_id, capacity)| (shift_idx, place_idx, *resource_id, capacity.clone()))
                })
                .flat_map(move |(shift_idx, place_idx, resource_id, capacity)| {
                    vehicle.vehicle_ids.iter().filter_map(move |vehicle_id| {
                        let job_id = format!("{vehicle_id}_reload_{shift_idx}_{place_idx}");
                        let capacity = capacity_map(capacity.clone());
                        job_index.get(&job_id).map(|job| (job.clone(), (capacity, resource_id)))
                    })
                })
        })
        .collect()
}

fn create_optional_break_feature(name: &str) -> GenericResult<Feature> {
    fn is_break_job(single: &Single) -> bool {
        single.dimens.get_job_type().map_or(false, |job_type| job_type == "break")
    }

    BreakFeatureBuilder::new(name)
        .set_violation_code(BREAK_CONSTRAINT_CODE)
        .set_is_break_single(is_break_job)
        .set_belongs_to_route(|route, job| {
            let Some(single) = job.as_single().filter(|single| is_break_job(single)) else { return false };
            is_correct_vehicle(route, single)
        })
        .set_policy(|single| single.dimens.get_break_policy().cloned().unwrap_or(BreakPolicy::SkipIfNoIntersection))
        .build()
}

fn get_tour_order_fn() -> TourOrderFn {
    TourOrderFn::Left(Arc::new(|single| {
        single.dimens.get_job_order().copied().map(|order| OrderResult::Value(order as f64)).unwrap_or_else(|| {
            single.dimens.get_job_type().map_or(OrderResult::Default, |v| {
                match v.as_str() {
                    "break" | "reload" => OrderResult::Ignored,
                    // job without value
                    _ => OrderResult::Default,
                }
            })
        })
    }))
}
