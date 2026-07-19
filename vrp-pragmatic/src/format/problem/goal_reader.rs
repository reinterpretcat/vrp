#[cfg(test)]
#[path = "../../../tests/unit/format/problem/goal_reader_test.rs"]
mod goal_reader_test;

use super::*;
use std::ops::Mul;
use vrp_core::algorithms::clustering::kmedoids::create_hierarchical_kmedoids;
use vrp_core::construction::clustering::vicinity::ClusterInfoDimension;
use vrp_core::construction::enablers::{FeatureCombinator, TotalDistanceTourState, TotalDurationTourState};
use vrp_core::construction::features::*;
// `TerritoryProximity` is ambiguous between this glob import and the pragmatic-format model's own
// `TerritoryProximity` (brought in via `use super::*;`), so the core type needs an explicit,
// disambiguating import — same reasoning as the `Location`/`Job` aliases below.
use vrp_core::construction::features::TerritoryProximity as CoreTerritoryProximity;
use vrp_core::algorithms::assignment::min_cost_assignment;
use vrp_core::construction::clustering::territory_seeds::build_balanced_territory_seeds;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::common::{Demand, Location as CoreLocation, LoadOps, MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::{Actor, DriverIdDimension, Job as CoreJob, Single, TransportCost};
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
        features.push(get_tour_limit_feature(
            "tour_limit",
            api_problem,
            blocks.transport.clone(),
            blocks.activity.clone(),
        )?)
    }

    if props.has_job_time_constraints {
        features.push(create_job_time_limits_feature(
            "job_time_limits",
            blocks.transport.clone(),
            blocks.activity.clone(),
            JOB_TIME_CONSTRAINT_CODE,
        )?)
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

    if props.has_min_vehicle_shifts
        && let Some(feature) = get_min_vehicle_shifts_feature("min_vehicle_shifts", api_problem)?
    {
        features.push(feature);
    }

    GoalContextBuilder::with_features(&features)?.set_main_goal(goal_builder.build()?).build()
}

/// Layer retains information about whether a feature is defined as standalone or as having some competitive.
enum FeatureLayer {
    /// A single feature layer: no competitive objectives.
    Single(Feature),

    /// A multi feature layer: multiple competitive objectives are available for multiple features.
    Multi { composition_type: MultiStrategy, features: Vec<Feature> },
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
            ViolationCode::unknown(),
        ),
        Objective::BalanceShifts { saturation, weight } => {
            const DEFAULT_VARIANCE_SATURATION: Float = 0.05;
            let saturation = saturation.unwrap_or(DEFAULT_VARIANCE_SATURATION).max(1e-6);
            let weight = weight.unwrap_or(1.).max(0.);
            let penalty = Arc::new(move |variance: Float| weight * variance / (variance + saturation));
            create_balance_shifts_feature_with_penalty("balance_shifts", penalty)
        }
        Objective::MinimizeUnassigned { breaks } => MinimizeUnassignedBuilder::new("min_unassigned")
            .set_job_estimator({
                let break_value = *breaks;
                let default_value = 1.;
                move |_, job| {
                    if let Some(clusters) = job.dimens().get_cluster_info() {
                        clusters.len() as Float * default_value
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
                        let mut max_ratio = Float::default();

                        for (idx, value) in capacity.load.iter().enumerate() {
                            let ratio = loaded.load[idx] as Float / *value as Float;
                            max_ratio = max_ratio.max(ratio);
                        }

                        max_ratio
                    },
                    |vehicle| vehicle.dimens.get_vehicle_capacity().expect("vehicle has no capacity defined"),
                )
            } else {
                create_max_load_balanced_feature::<SingleDimLoad>(
                    "max_load_balance",
                    |loaded, capacity| loaded.value as Float / capacity.value as Float,
                    |vehicle| vehicle.dimens.get_vehicle_capacity().expect("vehicle has no capacity defined"),
                )
            }
        }
        Objective::BalanceActivities => create_activity_balanced_feature("activity_balance"),
        Objective::BalanceDistance => create_distance_balanced_feature("distance_balance"),
        Objective::BalanceDuration => create_duration_balanced_feature("duration_balance"),
        Objective::BalanceProductionValue => {
            create_production_value_balanced_feature("production_value_balance", |job| {
                job.dimens().get_production_value().copied().unwrap_or(0.)
            })
        }
        Objective::BalancePeriod { metric } => {
            let group_capacities: HashMap<String, usize> =
                blocks.fleet.actors.iter().fold(HashMap::new(), |mut acc, actor| {
                    if let Some(vehicle_id) = actor.vehicle.dimens.get_vehicle_id() {
                        *acc.entry(vehicle_id.clone()).or_insert(0) += 1;
                    }
                    acc
                });
            let group_key_fn = |actor: &Actor| actor.vehicle.dimens.get_vehicle_id().cloned();

            // Fixed per-problem reference used to self-normalize the balance objective so its
            // weight in a scalarizing multi-objective is a dimensionless preference on the same
            // footing as compact-tour and vehicle-distance. It is the ideal work per shift: the
            // metric's ideal total spread evenly over every available shift.
            let total_capacity = group_capacities.values().sum::<usize>() as Float;
            let reference = (compute_period_reference(metric, blocks) / total_capacity.max(1.0)).max(1e-6);

            match metric {
                BalancePeriodMetric::Distance => create_period_balanced_feature(
                    "period_balance",
                    group_capacities,
                    group_key_fn,
                    |route_ctx| route_ctx.state().get_total_distance().copied().unwrap_or(0.),
                    reference,
                ),
                BalancePeriodMetric::Duration => create_period_balanced_feature(
                    "period_balance",
                    group_capacities,
                    group_key_fn,
                    |route_ctx| route_ctx.state().get_total_duration().copied().unwrap_or(0.),
                    reference,
                ),
                BalancePeriodMetric::Activities => create_period_balanced_feature(
                    "period_balance",
                    group_capacities,
                    group_key_fn,
                    |route_ctx| route_ctx.route().tour.job_activity_count() as Float,
                    reference,
                ),
                BalancePeriodMetric::ProductionValue => create_period_balanced_feature(
                    "period_balance",
                    group_capacities,
                    group_key_fn,
                    |route_ctx| {
                        route_ctx
                            .route()
                            .tour
                            .jobs()
                            .map(|job| job.dimens().get_production_value().copied().unwrap_or(0.))
                            .sum()
                    },
                    reference,
                ),
            }
        }
        Objective::CompactTour { job_radius } => {
            create_tour_compactness_feature("tour_compact", blocks.jobs.clone(), *job_radius)
        }
        Objective::TourOrder => create_tour_order_soft_feature("tour_order", get_tour_order_fn()),
        Objective::MinimizeTourSizeViolation => create_min_activity_limit_feature(
            "min_tour_size_objective",
            Arc::new(|actor| actor.vehicle.dimens.get_min_tour_size().copied()),
        ),
        Objective::FastService => get_fast_service_feature("fast_service", blocks),
        Objective::MinimizeOverdue => MinimizeOverdueBuilder::new("min_overdue")
            .set_job_due_date_fn(|job| {
                // For Multi jobs, find the earliest due date among all tasks
                // For Single jobs, just get the due date directly
                match job {
                    CoreJob::Single(single) => single.dimens.get_job_due_date().copied(),
                    CoreJob::Multi(multi) => multi
                        .jobs
                        .iter()
                        .filter_map(|single| single.dimens.get_job_due_date().copied())
                        .min_by(|a, b| a.total_cmp(b)),
                }
            })
            .set_scheduled_date_fn(|route_ctx| route_ctx.route().actor.detail.time.start)
            .set_unassigned_penalty_fn(|job| {
                // High penalty for unassigned jobs that have a due date
                let has_due_date = match job {
                    CoreJob::Single(single) => single.dimens.get_job_due_date().is_some(),
                    CoreJob::Multi(multi) => multi.jobs.iter().any(|single| single.dimens.get_job_due_date().is_some()),
                };
                if has_due_date { 10000.0 } else { 0.0 }
            })
            .build(),
        Objective::MinimizeVehicleDistance => VehicleDistanceFeatureBuilder::new("min_vehicle_distance")
            .set_transport(blocks.transport.clone())
            .set_actors(blocks.fleet.actors.clone())
            .set_jobs(blocks.jobs.clone())
            .set_compatibility_fn(territory_compatibility_fn())
            .build(),
        Objective::HierarchicalAreas { levels } => get_hierarchical_areas_feature(blocks, *levels),
        Objective::Territory { proximity, balance, anchors, allow_idle_drivers } => {
            let proximity = match proximity {
                model::TerritoryProximity::Distance => CoreTerritoryProximity::Distance,
                model::TerritoryProximity::Time => CoreTerritoryProximity::Time,
            };
            let balance = balance.as_ref().map(|m| match m {
                BalancePeriodMetric::Distance => TerritoryBalance::Distance,
                BalancePeriodMetric::Duration => TerritoryBalance::Duration,
                BalancePeriodMetric::Activities => TerritoryBalance::Activities,
                BalancePeriodMetric::ProductionValue => TerritoryBalance::ProductionValue,
            });
            // Empty anchors ⇒ derive balanced medoid seeds + power weights and match drivers to
            // them (Hungarian); explicit anchors keep the caller-supplied territory (no weights).
            // Anchor values are already routing-matrix indices; core Location is a usize.
            let (anchors, weights) = if anchors.is_empty() {
                derive_territory_anchors_and_weights(blocks, proximity, balance)
            } else {
                (anchors.iter().map(|(k, &idx)| (k.clone(), idx as CoreLocation)).collect(), HashMap::new())
            };

            TerritoryFeatureBuilder::new("territory")
                .set_transport(blocks.transport.clone())
                .set_actors(blocks.fleet.actors.clone())
                .set_jobs(blocks.jobs.clone())
                .set_proximity(proximity)
                .set_balance(balance)
                .set_anchors(anchors)
                .set_weights(weights)
                .set_allow_idle_drivers(*allow_idle_drivers)
                .set_job_value_fn(|job| job.dimens().get_production_value().copied().unwrap_or(0.))
                .set_compatibility_fn(territory_compatibility_fn())
                .build()
        }
        Objective::MultiObjective { objectives, strategy: composition_type } => {
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

fn get_hierarchical_areas_feature(blocks: &ProblemBlocks, levels: usize) -> GenericResult<Feature> {
    let locations = (0..blocks.transport.size()).collect::<Vec<_>>();
    let profile =
        blocks.fleet.profiles.first().cloned().ok_or_else(|| GenericError::from("should have at least one profile"))?;

    let clusters = create_hierarchical_kmedoids(&locations, levels, {
        let transport = blocks.transport.clone();
        move |from, to| transport.distance_approx(&profile, *from, *to)
    });

    let cost_feature = TransportFeatureBuilder::new("min_distance_hierarchical")
        .set_time_constrained(false)
        .set_transport_cost(blocks.transport.clone())
        .set_activity_cost(blocks.activity.clone())
        .build_minimize_distance()?;

    create_hierarchical_areas_feature(cost_feature, &clusters, {
        let transport = blocks.transport.clone();
        move |profile, from, to| transport.distance_approx(profile, from, to)
    })
}

/// The actor/job compatibility check shared by `MinimizeVehicleDistance` and `Territory`: an
/// actor may serve a job only if it has the required skills and its shift's time windows overlap
/// at least one of the job's time windows. The latter (day-availability) check matters because
/// both objectives compare a job against "the nearest compatible vehicle" — without it, that
/// comparison could be made against a vehicle that isn't even working the day the job needs to be
/// served.
fn territory_compatibility_fn() -> impl Fn(&CoreJob, &Actor) -> bool + Send + Sync + 'static {
    |job, actor| {
        if let Some(job_skills) = job.dimens().get_job_skills() {
            let vehicle_skills = actor.vehicle.dimens.get_vehicle_skills();
            if !is_job_skills_compatible(job_skills, &vehicle_skills) {
                return false;
            }
        }

        let actor_tw = &actor.detail.time;
        match job {
            CoreJob::Single(single) => {
                single.places.iter().flat_map(|p| p.times.iter()).any(|ts| ts.intersects(0.0, actor_tw))
            }
            CoreJob::Multi(multi) => multi
                .jobs
                .iter()
                .flat_map(|s| s.places.iter())
                .flat_map(|p| p.times.iter())
                .any(|ts| ts.intersects(0.0, actor_tw)),
        }
    }
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
                        // objectives will be combined below using [eval_multi_objective_strategy] function
                        .set_objective_combinator(|_| Ok(None))
                        .combine()?,
                );

                (all_features, eval_multi_objective_strategy(&objectives, composition_type, builder)?)
            }
        })
    })
}

fn eval_multi_objective_strategy(
    objectives: &[Arc<dyn FeatureObjective>],
    composition_type: &MultiStrategy,
    builder: GoalBuilder,
) -> GenericResult<GoalBuilder> {
    Ok(match composition_type {
        MultiStrategy::Sum => builder.add_multi(
            objectives,
            |os, a, b| dominance_order(a, b, os.iter().map(|o| |a, b| o.fitness(a).total_cmp(&o.fitness(b)))),
            |os, move_ctx| os.iter().map(|o| o.estimate(move_ctx)).sum(),
        ),

        MultiStrategy::WeightedSum { weights } => {
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
                |os, a, b| dominance_order(a, b, os.iter().map(|o| |a, b| o.fitness(a).total_cmp(&o.fitness(b)))),
                {
                    let weights = weights.clone();
                    move |os, move_ctx| os.iter().enumerate().map(|(idx, o)| o.estimate(move_ctx) * weights[idx]).sum()
                },
            )
        }

        MultiStrategy::WeightedSumScalar { weights } => {
            if objectives.len() != weights.len() {
                return Err(format!(
                    "weighted sum scalar requires same amount of weights as objective count: {} vs {}",
                    weights.len(),
                    objectives.len()
                )
                .into());
            }

            builder.add_multi(
                objectives,
                {
                    // Selection: collapse the weighted fitness values into a single scalar and
                    // compare those. This makes the weighted sum the quantity being minimized,
                    // instead of the Pareto-dominance order used by `WeightedSum`.
                    //
                    // Each fitness is divided by the objective's own `fitness_scale()` so that
                    // objectives with very different magnitudes combine on a comparable scale and
                    // the weights stay dimensionless preferences. The objective owns this scale;
                    // the strategy only applies it uniformly (default scale is 1.0 = no-op).
                    let weights = weights.clone();
                    move |os: &[Arc<dyn FeatureObjective>], a: &InsertionContext, b: &InsertionContext| {
                        let score = |s: &InsertionContext| -> Float {
                            os.iter().enumerate().map(|(idx, o)| weights[idx] * o.fitness(s) / o.fitness_scale()).sum()
                        };
                        score(a).total_cmp(&score(b))
                    }
                },
                {
                    // Insertion heuristic: same weighted, self-normalized estimate as selection.
                    let weights = weights.clone();
                    move |os, move_ctx| {
                        os.iter()
                            .enumerate()
                            .map(|(idx, o)| weights[idx] * o.estimate(move_ctx) / o.fitness_scale())
                            .sum()
                    }
                },
            )
        }
    })
}

/// Computes the ideal total of a period-balance metric for the whole problem: the best-case
/// magnitude the metric can take if every job is served ideally. Divided by the number of
/// available shifts, this yields the ideal per-shift load used as the balance objective's
/// fixed self-normalization reference.
fn compute_period_reference(metric: &BalancePeriodMetric, blocks: &ProblemBlocks) -> Float {
    match metric {
        BalancePeriodMetric::Activities => blocks
            .jobs
            .all()
            .iter()
            .map(|job| match job {
                CoreJob::Single(_) => 1.0,
                CoreJob::Multi(multi) => multi.jobs.len() as Float,
            })
            .sum(),
        BalancePeriodMetric::ProductionValue => blocks
            .jobs
            .all()
            .iter()
            .map(|job| job.dimens().get_production_value().copied().unwrap_or(0.))
            .sum(),
        BalancePeriodMetric::Distance => {
            compute_ideal_round_trip_total(blocks.transport.as_ref(), &blocks.fleet.actors, blocks.jobs.all(), false)
        }
        BalancePeriodMetric::Duration => {
            compute_ideal_round_trip_total(blocks.transport.as_ref(), &blocks.fleet.actors, blocks.jobs.all(), true)
        }
    }
}

/// Sums, over every job, the shortest round-trip from any vehicle's start depot to the job and
/// back (measured as distance or duration). This is the same "ideal travel" notion the
/// vehicle-distance feature uses, ignoring compatibility: it only needs to be a stable
/// order-of-magnitude reference, not a routed lower bound.
fn compute_ideal_round_trip_total(
    transport: &dyn TransportCost,
    actors: &[Arc<Actor>],
    jobs: &[CoreJob],
    use_duration: bool,
) -> Float {
    jobs.iter()
        .filter_map(|job| {
            let job_loc = job_primary_location(job)?;
            actors
                .iter()
                .filter_map(|actor| actor.detail.start.as_ref().map(|start| (start.location, &actor.vehicle.profile)))
                .map(|(start, profile)| {
                    if use_duration {
                        transport.duration_approx(profile, start, job_loc)
                            + transport.duration_approx(profile, job_loc, start)
                    } else {
                        transport.distance_approx(profile, start, job_loc)
                            + transport.distance_approx(profile, job_loc, start)
                    }
                })
                .min_by(|a, b| a.total_cmp(b))
        })
        .sum()
}

/// Iterations of static value-balancing applied to the derived territory seeds.
const TERRITORY_SEED_ITERATIONS: usize = 10;

/// Derives per-driver territory anchors + power weights from the problem when no explicit anchors
/// are configured: balanced medoid seeds over the customer jobs (compact by proximity, weighted so
/// each power cell captures ~equal production value), matched to drivers by start→seed proximity
/// via the Hungarian algorithm. Keyed like the core `driver_key` (driver id, else vehicle id).
/// Returns empty maps when there is nothing to derive (no drivers, no jobs, no profile).
fn derive_territory_anchors_and_weights(
    blocks: &ProblemBlocks,
    proximity: CoreTerritoryProximity,
    balance: Option<TerritoryBalance>,
) -> (HashMap<String, CoreLocation>, HashMap<String, Float>) {
    let Some(profile) = blocks.fleet.profiles.first().cloned() else {
        return (HashMap::new(), HashMap::new());
    };
    let transport = blocks.transport.clone();
    let dist = move |a: CoreLocation, b: CoreLocation| match proximity {
        CoreTerritoryProximity::Distance => transport.distance_approx(&profile, a, b),
        CoreTerritoryProximity::Time => transport.duration_approx(&profile, a, b),
    };

    // Customer jobs as (location, balance-metric value): the seeds are balanced on the SAME metric
    // the territory objective balances, so derived territories equalize whatever the caller asked
    // for (stops, production value, duration; distance/none fall back to job count).
    let jobs: Vec<(CoreLocation, Float)> = blocks
        .jobs
        .all()
        .iter()
        .filter_map(|job| job_primary_location(job).map(|loc| (loc, territory_job_metric(job, balance))))
        .collect();

    // Distinct drivers in first-seen order, each with a representative start location.
    let mut driver_order: Vec<String> = Vec::new();
    let mut driver_start: HashMap<String, CoreLocation> = HashMap::new();
    for actor in blocks.fleet.actors.iter() {
        let key = actor
            .vehicle
            .dimens
            .get_driver_id()
            .cloned()
            .or_else(|| actor.vehicle.dimens.get_vehicle_id().cloned())
            .unwrap_or_default();
        driver_start.entry(key.clone()).or_insert_with(|| {
            driver_order.push(key.clone());
            actor.detail.start.as_ref().map(|p| p.location).unwrap_or(0)
        });
    }

    let k = driver_order.len();
    if k == 0 || jobs.is_empty() {
        return (HashMap::new(), HashMap::new());
    }

    let seeds = build_balanced_territory_seeds(&jobs, k, dist.clone(), TERRITORY_SEED_ITERATIONS);
    let s = seeds.len();
    if s == 0 {
        return (HashMap::new(), HashMap::new());
    }

    // Hungarian match drivers → seeds by start→seed proximity. When there are fewer seeds than
    // drivers, pad the cost matrix to k×k with high-cost dummy columns so the real seeds are filled
    // first and the leftover drivers are simply left unanchored.
    let mut max_cost = 0.0f64;
    let mut cost: Vec<Vec<f64>> = driver_order
        .iter()
        .map(|key| {
            let start = driver_start[key];
            seeds
                .iter()
                .map(|seed| {
                    let c = dist(start, seed.location);
                    max_cost = max_cost.max(c);
                    c
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let big = (max_cost + 1.0) * (k as f64 + 1.0);
    for row in cost.iter_mut() {
        row.resize(k, big);
    }

    let assign = min_cost_assignment(&cost);
    let mut anchors = HashMap::new();
    let mut weights = HashMap::new();
    for (d, key) in driver_order.iter().enumerate() {
        let col = assign[d];
        if col < s {
            anchors.insert(key.clone(), seeds[col].location);
            weights.insert(key.clone(), seeds[col].weight);
        }
    }
    (anchors, weights)
}

fn job_primary_location(job: &CoreJob) -> Option<CoreLocation> {
    match job {
        CoreJob::Single(single) => single.places.first().and_then(|place| place.location),
        CoreJob::Multi(multi) => multi.jobs.first().and_then(|single| single.places.first().and_then(|p| p.location)),
    }
}

/// The per-job quantity the derived territory seeds balance on, matching the territory objective's
/// `balance`: production value, service duration, or job count (activities / distance / unspecified).
fn territory_job_metric(job: &CoreJob, balance: Option<TerritoryBalance>) -> Float {
    match balance {
        Some(TerritoryBalance::ProductionValue) => job.dimens().get_production_value().copied().unwrap_or(0.),
        Some(TerritoryBalance::Duration) => job_service_duration(job),
        // Activities, Distance, or no balance → equalize job count per territory.
        _ => 1.0,
    }
}

/// A job's total service time (its primary place duration, summed over sub-jobs for a multi-job).
fn job_service_duration(job: &CoreJob) -> Float {
    match job {
        CoreJob::Single(single) => single.places.first().map(|p| p.duration).unwrap_or(0.),
        CoreJob::Multi(multi) => multi.jobs.iter().filter_map(|s| s.places.first().map(|p| p.duration)).sum(),
    }
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
        .set_is_filtered_job(|job| job.dimens().get_job_type().is_some_and(|job_type| job_type == "reload"))
        .build()
}

fn create_capacity_with_reload_feature<T: LoadOps + SharedResource + Mul<Float, Output = T>>(
    name: &str,
    api_problem: &ApiProblem,
    blocks: &ProblemBlocks,
    capacity_map: fn(Vec<i32>) -> T,
) -> GenericResult<Feature> {
    const RELOAD_THRESHOLD: Float = 0.9;

    fn is_reload_single(single: &Single) -> bool {
        single.dimens.get_job_type().is_some_and(|job_type| job_type == "reload")
    }

    let builder = ReloadFeatureFactory::new(name)
        .set_capacity_code(CAPACITY_CONSTRAINT_CODE)
        .set_load_schedule_threshold(move |capacity: &T| *capacity * RELOAD_THRESHOLD)
        .set_is_reload_single(is_reload_single)
        .set_belongs_to_route(|route: &Route, job: &CoreJob| {
            job.as_single().is_some_and(|single| is_reload_single(single.as_ref()) && is_correct_vehicle(route, single))
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
    transport: Arc<dyn TransportCost>,
    activity: Arc<dyn ActivityCost>,
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

    let get_limit = |limit_map: HashMap<String, Float>| {
        Arc::new(move |actor: &Actor| {
            actor.vehicle.dimens.get_vehicle_type().and_then(|v_type| limit_map.get(v_type)).cloned()
        })
    };

    create_travel_limit_feature(
        name,
        transport,
        activity,
        DISTANCE_LIMIT_CONSTRAINT_CODE,
        DURATION_LIMIT_CONSTRAINT_CODE,
        get_limit(distances),
        get_limit(durations),
    )
}

fn get_min_vehicle_shifts_feature(name: &str, api_problem: &ApiProblem) -> GenericResult<Option<Feature>> {
    let requirements = api_problem
        .fleet
        .vehicles
        .iter()
        .filter_map(|vehicle| vehicle.min_shifts.as_ref().map(|value| (vehicle, value.clone())))
        .flat_map(|(vehicle, min_shifts)| {
            vehicle.vehicle_ids.iter().map(move |vehicle_id| {
                (
                    vehicle_id.clone(),
                    MinShiftRequirement { minimum: min_shifts.value, allow_zero_usage: min_shifts.allow_zero_usage },
                )
            })
        })
        .collect::<HashMap<_, _>>();

    if requirements.is_empty() {
        return Ok(None);
    }

    MinVehicleShiftsFeatureBuilder::new(name)
        .with_violation_code(MIN_VEHICLE_SHIFTS_CONSTRAINT_CODE)
        .with_requirements(requirements)
        .build()
        .map(Some)
}

fn get_recharge_feature(
    name: &str,
    api_problem: &ApiProblem,
    transport: Arc<dyn TransportCost>,
) -> GenericResult<Feature> {
    fn is_recharge_single(single: &Single) -> bool {
        single.dimens.get_job_type().is_some_and(|job_type| job_type == "recharge")
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
                .is_some_and(|single| is_recharge_single(single.as_ref()) && is_correct_vehicle(route, single))
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
        single.dimens.get_job_type().is_some_and(|job_type| job_type == "break")
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
        single.dimens.get_job_order().copied().map(|order| OrderResult::Value(order as Float)).unwrap_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_problem_with_min_shifts(min_shifts: Option<VehicleMinShifts>) -> ApiProblem {
        ApiProblem {
            plan: Plan { jobs: vec![], relations: None, clustering: None },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    type_id: "vehicle_type".to_string(),
                    vehicle_ids: vec!["vehicle_1".to_string()],
                    profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                    costs: VehicleCosts { fixed: Some(0.), distance: 1., time: 1., span: None },
                    shifts: vec![VehicleShift {
                        start: ShiftStart {
                            earliest: "1970-01-01T00:00:00Z".to_string(),
                            latest: None,
                            location: Location::new_coordinate(0., 0.),
                        },
                        end: None,
                        breaks: None,
                        reloads: None,
                        recharges: None,
                        job_times: None,
                    }],
                    capacity: vec![1],
                    skills: None,
                    limits: None,
                    min_shifts,
                    driver_id: None,
                }],
                profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
                resources: None,
            },
            objectives: None,
        }
    }

    #[test]
    fn creates_min_vehicle_shift_feature_when_needed() {
        let problem = create_problem_with_min_shifts(Some(VehicleMinShifts { value: 1, allow_zero_usage: false }));
        let feature = get_min_vehicle_shifts_feature("min_vehicle_shifts", &problem).unwrap();

        assert!(feature.is_some());
    }

    #[test]
    fn returns_none_when_no_requirements() {
        let problem = create_problem_with_min_shifts(None);
        assert!(get_min_vehicle_shifts_feature("min_vehicle_shifts", &problem).unwrap().is_none());
    }
}
