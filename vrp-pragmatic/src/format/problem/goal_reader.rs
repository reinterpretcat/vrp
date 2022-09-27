use super::*;
use crate::construction::enablers::VehicleTie;
use crate::construction::features::*;
use vrp_core::construction::features::*;
use vrp_core::models::common::{LoadOps, MultiDimLoad, SingleDimLoad};
use vrp_core::models::problem::{ActivityCost, Actor, Jobs, TransportCost};
use vrp_core::models::{Feature, GoalContext, Lock};

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_goal_context(
    api_problem: &ApiProblem,
    jobs: &Jobs,
    job_index: &JobIndex,
    fleet: &CoreFleet,
    transport: Arc<dyn TransportCost + Send + Sync>,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    props: &ProblemProperties,
    locks: &[Arc<Lock>],
) -> Result<GoalContext, String> {
    let mut features = Vec::new();

    if props.has_unreachable_locations {
        features.push(create_reachable_feature("reachable", transport.clone(), RELOAD_RESOURCE_CONSTRAINT_CODE)?)
    }

    // TODO pick only one based on objectives
    features.push(create_minimize_transport_costs_feature(
        "transport",
        transport.clone(),
        activity.clone(),
        TIME_CONSTRAINT_CODE,
    )?);

    features.push(get_capacity_feature("capacity", api_problem, jobs, job_index, props)?);

    if props.has_tour_travel_limits {
        features.push(get_tour_limit_feature("tour_limit", api_problem, transport.clone())?)
    }

    if props.has_breaks {
        features.push(create_optional_break_feature("break", BREAK_CONSTRAINT_CODE)?)
    }

    if props.has_compatibility {
        features.push(create_compatibility_feature("compatibility", COMPATIBILITY_CONSTRAINT_CODE, COMPATIBILITY_KEY)?);
    }

    if props.has_group {
        features.push(create_group_feature("group", jobs.size(), GROUP_CONSTRAINT_CODE, GROUP_KEY)?);
    }

    if props.has_skills {
        features.push(create_skills_feature("skills", SKILL_CONSTRAINT_CODE)?)
    }

    if props.has_dispatch {
        features.push(create_dispatch_feature("dispatch", DISPATCH_CONSTRAINT_CODE)?)
    }

    if !locks.is_empty() {
        features.push(create_locked_jobs_feature("locked_jobs", fleet, locks, LOCKING_CONSTRAINT_CODE)?);
    }

    if props.has_tour_size_limits {
        features.push(create_activity_limit_feature(
            "activity_limit",
            TOUR_SIZE_CONSTRAINT_CODE,
            Arc::new(|actor| actor.vehicle.dimens.get_tour_size()),
        )?);
    }

    let (global_objective_map, local_objective_map) = get_objective_maps(api_problem)?;

    GoalContext::new(features.as_slice(), global_objective_map.as_slice(), local_objective_map.as_slice())
}

fn get_capacity_feature(
    name: &str,
    api_problem: &ApiProblem,
    jobs: &Jobs,
    job_index: &JobIndex,
    props: &ProblemProperties,
) -> Result<Feature, String> {
    if props.has_reloads {
        let threshold = 0.9;

        if props.has_multi_dimen_capacity {
            get_capacity_with_reload_feature::<MultiDimLoad>(
                name,
                api_problem,
                jobs,
                job_index,
                MultiDimLoad::new,
                Box::new(move |capacity| *capacity * threshold),
            )
        } else {
            get_capacity_with_reload_feature::<SingleDimLoad>(
                name,
                api_problem,
                jobs,
                job_index,
                |capacity| SingleDimLoad::new(capacity.first().cloned().unwrap_or_default()),
                Box::new(move |capacity| *capacity * threshold),
            )
        }
    } else {
        if props.has_multi_dimen_capacity {
            create_capacity_limit_feature::<MultiDimLoad>(name, CAPACITY_CONSTRAINT_CODE)
        } else {
            create_capacity_limit_feature::<SingleDimLoad>(name, CAPACITY_CONSTRAINT_CODE)
        }
    }
}

fn get_capacity_with_reload_feature<T: LoadOps + SharedResource>(
    name: &str,
    api_problem: &ApiProblem,
    jobs: &Jobs,
    job_index: &JobIndex,
    capacity_map: fn(Vec<i32>) -> T,
    load_schedule_threshold_fn: LoadScheduleThresholdFn<T>,
) -> Result<Feature, String> {
    let reload_resources = get_reload_resources(api_problem, job_index, capacity_map);
    let capacity_feature_factory: CapacityFeatureFactoryFn<T> = Box::new(|name, multi_trip| {
        create_capacity_limit_with_multi_trip_feature(name, CAPACITY_CONSTRAINT_CODE, multi_trip)
    });

    if reload_resources.is_empty() {
        create_simple_reload_multi_trip_feature(name, capacity_feature_factory, load_schedule_threshold_fn)
    } else {
        create_shared_reload_multi_trip_feature(
            name,
            capacity_feature_factory,
            load_schedule_threshold_fn,
            reload_resources,
            jobs.size(),
            RELOAD_RESOURCE_CONSTRAINT_CODE,
            RELOAD_RESOURCE_KEY,
        )
    }
}

fn get_tour_limit_feature(
    name: &str,
    api_problem: &ApiProblem,
    transport: Arc<dyn TransportCost + Send + Sync>,
) -> Result<Feature, String> {
    let (distances, durations) = api_problem
        .fleet
        .vehicles
        .iter()
        .filter_map(|vehicle| vehicle.limits.as_ref().map(|limits| (vehicle, limits)))
        .fold((HashMap::new(), HashMap::new()), |(mut distances, mut durations), (vehicle, limits)| {
            limits.max_distance.iter().for_each(|max_distance| {
                distances.insert(vehicle.type_id.clone(), *max_distance);
            });

            limits.shift_time.iter().for_each(|shift_time| {
                durations.insert(vehicle.type_id.clone(), *shift_time);
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
        get_limit(distances),
        get_limit(durations),
        DISTANCE_LIMIT_CONSTRAINT_CODE,
        DURATION_LIMIT_CONSTRAINT_CODE,
    )
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
                        let job_id = format!("{}_reload_{}_{}", vehicle_id, shift_idx, place_idx);
                        let capacity = capacity_map(capacity.clone());
                        job_index.get(&job_id).map(|job| (job.clone(), (capacity, resource_id)))
                    })
                })
        })
        .collect()
}

fn get_objective_maps(_: &ApiProblem) -> Result<(Vec<Vec<String>>, Vec<Vec<String>>), String> {
    unimplemented!()
}
