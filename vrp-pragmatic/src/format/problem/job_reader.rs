use crate::extensions::MultiDimensionalCapacity;
use crate::format::coord_index::CoordIndex;
use crate::format::problem::reader::{add_skills, parse_time_window, ApiProblem, JobIndex, ProblemProperties};
use crate::format::problem::{JobTask, RelationType, VehicleBreak, VehicleBreakTime, VehicleCargoPlace, VehicleType};
use crate::format::Location;
use crate::utils::VariableJobPermutation;
use std::collections::HashMap;
use std::sync::Arc;
use vrp_core::construction::constraints::{Demand, DemandDimension};
use vrp_core::models::common::{Dimensions, Duration, IdDimension, TimeOffset, TimeSpan, TimeWindow, ValueDimension};
use vrp_core::models::problem::{Actor, Fleet, Job, Jobs, Multi, Place, Single, TransportCost};
use vrp_core::models::{Lock, LockDetail, LockOrder, LockPosition};

// TODO configure sample size
const MULTI_JOB_SAMPLE_SIZE: usize = 3;

pub(crate) fn read_jobs_with_extra_locks(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    fleet: &Fleet,
    transport: &Arc<dyn TransportCost + Sync + Send>,
    job_index: &mut JobIndex,
) -> (Jobs, Vec<Arc<Lock>>) {
    let (mut jobs, mut locks) = read_required_jobs(api_problem, props, coord_index, job_index);
    let (conditional_jobs, conditional_locks) = read_conditional_jobs(api_problem, coord_index, job_index);

    jobs.extend(conditional_jobs);
    locks.extend(conditional_locks);

    (Jobs::new(fleet, jobs, transport), locks)
}

pub fn read_locks(api_problem: &ApiProblem, job_index: &JobIndex) -> Vec<Arc<Lock>> {
    if api_problem.plan.relations.as_ref().map_or(true, |r| r.is_empty()) {
        return vec![];
    }

    let relations = api_problem.plan.relations.as_ref().unwrap().iter().fold(HashMap::new(), |mut acc, r| {
        let shift_index = r.shift_index.unwrap_or(0);
        acc.entry((r.vehicle_id.clone(), shift_index)).or_insert_with(Vec::new).push(r.clone());

        acc
    });

    relations.into_iter().fold(vec![], |mut acc, ((vehicle_id, shift_index), rels)| {
        let condition = create_condition(vehicle_id.clone(), shift_index);
        let details = rels.iter().fold(vec![], |mut acc, rel| {
            let order = match rel.type_field {
                RelationType::Any => LockOrder::Any,
                RelationType::Sequence => LockOrder::Sequence,
                RelationType::Strict => LockOrder::Strict,
            };

            let position = match (rel.jobs.first().map(|s| s.as_str()), rel.jobs.last().map(|s| s.as_str())) {
                (Some("departure"), Some("arrival")) => LockPosition::Fixed,
                (Some("departure"), _) => LockPosition::Departure,
                (_, Some("arrival")) => LockPosition::Arrival,
                _ => LockPosition::Any,
            };

            let (_, _, jobs) = rel
                .jobs
                .iter()
                .filter(|job| job.as_str() != "departure" && job.as_str() != "arrival")
                .fold((0_usize, 0_usize, vec![]), |(mut break_idx, mut reload_idx, mut jobs), job| {
                    let job = match job.as_str() {
                        "break" => {
                            break_idx += 1;
                            let break_id = format!("{}_break_{}_{}", vehicle_id, shift_index, break_idx);
                            job_index.get(&break_id).cloned().unwrap()
                        }
                        "reload" => {
                            reload_idx += 1;
                            let reload_id = format!("{}_reload_{}_{}", vehicle_id, shift_index, reload_idx);
                            job_index.get(&reload_id).cloned().unwrap()
                        }
                        _ => job_index.get(job).unwrap().clone(),
                    };

                    jobs.push(job);

                    (break_idx, reload_idx, jobs)
                });

            acc.push(LockDetail::new(order, position, jobs));

            acc
        });

        acc.push(Arc::new(Lock::new(condition, details, false)));

        acc
    })
}

fn read_required_jobs(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> (Vec<Job>, Vec<Arc<Lock>>) {
    let mut jobs = vec![];
    let has_multi_dimens = props.has_multi_dimen_capacity;

    let get_single_from_task = |task: &JobTask, activity_type: &str, is_static_demand: bool| {
        let absent = (empty(), empty());
        let capacity = task.demand.clone().map_or_else(empty, MultiDimensionalCapacity::new);
        let demand = if is_static_demand { (capacity, empty()) } else { (empty(), capacity) };

        let demand = match activity_type {
            "pickup" => Demand { pickup: demand, delivery: absent },
            "delivery" => Demand { pickup: absent, delivery: demand },
            "replacement" => Demand { pickup: demand, delivery: demand },
            "service" => Demand { pickup: absent, delivery: absent },
            _ => panic!("Invalid activity type."),
        };

        let places =
            task.places.iter().map(|p| (Some(p.location.clone()), p.duration, parse_times(&p.times))).collect();

        get_single_with_extras(places, demand, &task.tag, activity_type, has_multi_dimens, &coord_index)
    };

    api_problem.plan.jobs.iter().for_each(|job| {
        let pickups = job.pickups.as_ref().map_or(0, |p| p.len());
        let deliveries = job.deliveries.as_ref().map_or(0, |p| p.len());
        let is_static_demand = pickups == 0 || deliveries == 0;

        let singles =
            job.pickups
                .iter()
                .flat_map(|tasks| tasks.iter().map(|task| get_single_from_task(task, "pickup", is_static_demand)))
                .chain(job.deliveries.iter().flat_map(|tasks| {
                    tasks.iter().map(|task| get_single_from_task(task, "delivery", is_static_demand))
                }))
                .chain(
                    job.replacements
                        .iter()
                        .flat_map(|tasks| tasks.iter().map(|task| get_single_from_task(task, "replacement", true))),
                )
                .chain(
                    job.services
                        .iter()
                        .flat_map(|tasks| tasks.iter().map(|task| get_single_from_task(task, "service", false))),
                )
                .collect::<Vec<_>>();

        assert!(!singles.is_empty());

        let problem_job = if singles.len() > 1 {
            get_multi_job(&job.id, job.priority, &job.skills, singles, job.pickups.as_ref().map_or(0, |p| p.len()))
        } else {
            get_single_job(&job.id, singles.into_iter().next().unwrap(), job.priority, &job.skills)
        };

        job_index.insert(job.id.clone(), problem_job.clone());
        jobs.push(problem_job);
    });

    (jobs, vec![])
}

fn read_conditional_jobs(
    api_problem: &ApiProblem,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> (Vec<Job>, Vec<Arc<Lock>>) {
    let mut jobs = vec![];
    let locks = vec![];
    api_problem.fleet.vehicles.iter().for_each(|vehicle| {
        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            if let Some(depots) = &shift.depots {
                read_depots(coord_index, job_index, &mut jobs, vehicle, shift_index, depots);
            }

            if let Some(breaks) = &shift.breaks {
                read_breaks(coord_index, job_index, &mut jobs, vehicle, shift_index, breaks);
            }

            if let Some(reloads) = &shift.reloads {
                read_reloads(coord_index, job_index, &mut jobs, vehicle, shift_index, reloads);
            }
        }
    });

    (jobs, locks)
}

fn read_breaks(
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
    jobs: &mut Vec<Job>,
    vehicle: &VehicleType,
    shift_index: usize,
    breaks: &[VehicleBreak],
) {
    (1..)
        .zip(breaks.iter())
        .flat_map(|(break_idx, place)| {
            vehicle
                .vehicle_ids
                .iter()
                .map(|vehicle_id| {
                    let times = match &place.time {
                        VehicleBreakTime::TimeWindow(time) if time.len() != 2 => {
                            panic!("Break with invalid time window specified: must have start and end!")
                        }
                        VehicleBreakTime::TimeOffset(offsets) if offsets.len() != 2 => {
                            panic!("Break with invalid offset specified: must have start and end!")
                        }
                        VehicleBreakTime::TimeWindow(time) => vec![TimeSpan::Window(parse_time_window(time))],
                        VehicleBreakTime::TimeOffset(offset) => {
                            vec![TimeSpan::Offset(TimeOffset::new(*offset.first().unwrap(), *offset.last().unwrap()))]
                        }
                    };

                    let job_id = format!("{}_break_{}_{}", vehicle_id, shift_index, break_idx);
                    let places = if let Some(locations) = &place.locations {
                        assert!(!locations.is_empty());
                        locations
                            .iter()
                            .map(|location| (Some(location.clone()), place.duration, times.clone()))
                            .collect()
                    } else {
                        vec![(None, place.duration, times)]
                    };

                    let job = get_conditional_job(coord_index, vehicle_id.clone(), "break", shift_index, places, &None);

                    (job_id, job)
                })
                .collect::<Vec<_>>()
        })
        .for_each(|(job_id, single)| add_conditional_job(job_index, jobs, job_id, single));
}

fn read_depots(
    _coord_index: &CoordIndex,
    _job_index: &mut JobIndex,
    _jobs: &mut Vec<Job>,
    _vehicle: &VehicleType,
    _shift_index: usize,
    _depots: &[VehicleCargoPlace],
) {
    // TODO
}

fn read_reloads(
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
    jobs: &mut Vec<Job>,
    vehicle: &VehicleType,
    shift_index: usize,
    reloads: &[VehicleCargoPlace],
) {
    (1..)
        .zip(reloads.iter())
        .flat_map(|(place_idx, place)| {
            vehicle
                .vehicle_ids
                .iter()
                .map(|vehicle_id| {
                    let job_id = format!("{}_reload_{}_{}", vehicle_id, shift_index, place_idx);
                    let times = parse_times(&place.times);

                    let job = get_conditional_job(
                        coord_index,
                        vehicle_id.clone(),
                        "reload",
                        shift_index,
                        vec![(Some(place.location.clone()), place.duration, times)],
                        &place.tag,
                    );

                    (job_id, job)
                })
                .collect::<Vec<_>>()
        })
        .for_each(|(job_id, single)| {
            add_conditional_job(job_index, jobs, job_id, single);
        });
}

fn get_conditional_job(
    coord_index: &CoordIndex,
    vehicle_id: String,
    job_type: &str,
    shift_index: usize,
    places: Vec<(Option<Location>, Duration, Vec<TimeSpan>)>,
    tag: &Option<String>,
) -> Single {
    let mut single = get_single(places, coord_index);
    single.dimens.set_id(job_type);
    single.dimens.set_value("type", job_type.to_string());
    single.dimens.set_value("shift_index", shift_index);
    single.dimens.set_value("vehicle_id", vehicle_id);
    if let Some(tag) = tag {
        single.dimens.set_value("tag", tag.clone());
    }

    single
}

fn add_conditional_job(job_index: &mut JobIndex, jobs: &mut Vec<Job>, job_id: String, single: Single) {
    let job = Job::Single(Arc::new(single));
    job_index.insert(job_id, job.clone());
    jobs.push(job);
}

fn get_single(places: Vec<(Option<Location>, Duration, Vec<TimeSpan>)>, coord_index: &CoordIndex) -> Single {
    Single {
        places: places
            .into_iter()
            .map(|(location, duration, times)| Place {
                location: location.as_ref().and_then(|l| coord_index.get_by_loc(l)),
                duration,
                times,
            })
            .collect(),
        dimens: Default::default(),
    }
}

fn get_single_with_extras(
    places: Vec<(Option<Location>, Duration, Vec<TimeSpan>)>,
    demand: Demand<MultiDimensionalCapacity>,
    tag: &Option<String>,
    activity_type: &str,
    has_multi_dimens: bool,
    coord_index: &CoordIndex,
) -> Single {
    let mut single = get_single(places, coord_index);
    if has_multi_dimens {
        single.dimens.set_demand(demand);
    } else {
        single.dimens.set_demand(Demand {
            pickup: (demand.pickup.0.capacity[0], demand.pickup.1.capacity[0]),
            delivery: (demand.delivery.0.capacity[0], demand.delivery.1.capacity[0]),
        });
    }
    single.dimens.set_value("type", activity_type.to_string());
    add_tag(&mut single.dimens, tag);

    single
}

fn get_single_job(id: &str, single: Single, priority: Option<i32>, skills: &Option<Vec<String>>) -> Job {
    let mut single = single;
    single.dimens.set_id(id);

    add_priority(&mut single.dimens, priority);
    add_skills(&mut single.dimens, skills);

    Job::Single(Arc::new(single))
}

fn get_multi_job(
    id: &str,
    priority: Option<i32>,
    skills: &Option<Vec<String>>,
    singles: Vec<Single>,
    deliveries_start_index: usize,
) -> Job {
    let mut dimens: Dimensions = Default::default();
    dimens.set_id(id);
    add_priority(&mut dimens, priority);
    add_skills(&mut dimens, skills);

    let singles = singles.into_iter().map(Arc::new).collect::<Vec<_>>();

    let multi = if singles.len() == 2 && deliveries_start_index == 1 {
        Multi::new(singles, dimens)
    } else {
        let jobs_len = singles.len();
        Multi::new_with_permutator(
            singles,
            dimens,
            Box::new(VariableJobPermutation::new(jobs_len, deliveries_start_index, MULTI_JOB_SAMPLE_SIZE)),
        )
    };

    Job::Multi(Multi::bind(multi))
}

fn create_condition(vehicle_id: String, shift_index: usize) -> Arc<dyn Fn(&Actor) -> bool + Sync + Send> {
    Arc::new(move |actor: &Actor| {
        *actor.vehicle.dimens.get_id().unwrap() == vehicle_id
            && *actor.vehicle.dimens.get_value::<usize>("shift_index").unwrap() == shift_index
    })
}

fn add_tag(dimens: &mut Dimensions, tag: &Option<String>) {
    if let Some(tag) = tag {
        dimens.set_value("tag", tag.clone());
    }
}

fn add_priority(dimens: &mut Dimensions, priority: Option<i32>) {
    if let Some(priority) = priority {
        dimens.set_value("priority", priority);
    }
}

fn empty() -> MultiDimensionalCapacity {
    MultiDimensionalCapacity::default()
}

fn parse_times(times: &Option<Vec<Vec<String>>>) -> Vec<TimeSpan> {
    times.as_ref().map_or(vec![TimeSpan::Window(TimeWindow::max())], |tws| {
        tws.iter().map(|tw| TimeSpan::Window(parse_time_window(tw))).collect()
    })
}
