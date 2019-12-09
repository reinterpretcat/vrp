use crate::extensions::MultiDimensionalCapacity;
use crate::json::coord_index::CoordIndex;
use crate::json::problem::reader::{add_skills, parse_time_window, ApiProblem, JobIndex, ProblemProperties};
use crate::json::problem::{JobVariant, RelationType, VehicleBreak, VehicleType};
use crate::utils::get_split_permutations;
use core::construction::constraints::{Demand, DemandDimension};
use core::models::common::{Dimensions, Duration, IdDimension, TimeWindow, ValueDimension};
use core::models::problem::{Actor, Fleet, Job, Jobs, Multi, Place, Single, TransportCost};
use core::models::{Lock, LockDetail, LockOrder, LockPosition};
use std::collections::HashMap;
use std::sync::Arc;

// TODO configure sample size
const MULTI_JOB_SAMPLE_SIZE: usize = 3;

pub fn read_jobs_with_extra_locks(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    fleet: &Fleet,
    transport: &impl TransportCost,
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
        acc.entry((r.vehicle_id.clone(), shift_index)).or_insert(vec![]).push(r.clone());

        acc
    });

    let locks = relations.into_iter().fold(vec![], |mut acc, ((vehicle_id, shift_index), rels)| {
        let vehicle_id_copy = vehicle_id.clone();
        let condition = create_condition(vehicle_id_copy, shift_index);
        let details = rels.iter().fold(vec![], |mut acc, rel| {
            let order = match rel.type_field {
                RelationType::Tour => LockOrder::Any,
                RelationType::Flexible => LockOrder::Sequence,
                RelationType::Sequence => LockOrder::Strict,
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
                            job_index.get(format!("{}_break_{}", vehicle_id, break_idx).as_str()).cloned().unwrap()
                        }
                        "reload" => {
                            reload_idx += 1;
                            job_index.get(format!("{}_reload_{}", vehicle_id, reload_idx).as_str()).cloned().unwrap()
                        }
                        _ => job_index.get(job).unwrap().clone(),
                    };

                    jobs.push(job);

                    (break_idx, reload_idx, jobs)
                });

            acc.push(LockDetail::new(order, position, jobs));

            acc
        });

        acc.push(Arc::new(Lock::new(condition, details)));

        acc
    });

    locks
}

fn read_required_jobs(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> (Vec<Arc<Job>>, Vec<Arc<Lock>>) {
    let mut jobs = vec![];
    api_problem.plan.jobs.iter().for_each(|job| match job {
        JobVariant::Single(job) => {
            let demand = MultiDimensionalCapacity::new(job.demand.clone());
            let is_shipment = job.places.pickup.is_some() && job.places.delivery.is_some();
            let demand = if is_shipment { (empty(), demand) } else { (demand, empty()) };

            let pickup = job.places.pickup.as_ref().map(|pickup| {
                get_single_with_extras(
                    &pickup.location,
                    pickup.duration,
                    Demand { pickup: demand, delivery: (empty(), empty()) },
                    &pickup.times,
                    &pickup.tag,
                    "pickup",
                    props.has_multi_dimen_capacity,
                    &coord_index,
                )
            });
            let delivery = job.places.delivery.as_ref().map(|delivery| {
                get_single_with_extras(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (empty(), empty()), delivery: demand },
                    &delivery.times,
                    &delivery.tag,
                    "delivery",
                    props.has_multi_dimen_capacity,
                    &coord_index,
                )
            });

            let problem_job = match (pickup, delivery) {
                (Some(pickup), Some(delivery)) => {
                    get_multi_job(&job.id, &job.skills, vec![Arc::new(pickup), Arc::new(delivery)], 1)
                }
                (Some(pickup), None) => get_single_job(&job.id, pickup, &job.skills),
                (None, Some(delivery)) => get_single_job(&job.id, delivery, &job.skills),
                (None, None) => panic!("Single job should contain pickup and/or delivery."),
            };

            job_index.insert(job.id.clone(), problem_job.clone());
            jobs.push(problem_job);
        }
        JobVariant::Multi(job) => {
            let mut singles = job
                .places
                .pickups
                .iter()
                .map(|pickup| {
                    let demand = MultiDimensionalCapacity::new(pickup.demand.clone());
                    Arc::new(get_single_with_extras(
                        &pickup.location,
                        pickup.duration,
                        Demand { pickup: (empty(), demand), delivery: (empty(), empty()) },
                        &pickup.times,
                        &pickup.tag,
                        "pickup",
                        props.has_multi_dimen_capacity,
                        &coord_index,
                    ))
                })
                .collect::<Vec<Arc<Single>>>();
            singles.extend(job.places.deliveries.iter().map(|delivery| {
                let demand = MultiDimensionalCapacity::new(delivery.demand.clone());
                Arc::new(get_single_with_extras(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (empty(), empty()), delivery: (empty(), demand) },
                    &delivery.times,
                    &delivery.tag,
                    "delivery",
                    props.has_multi_dimen_capacity,
                    &coord_index,
                ))
            }));

            let problem_job = get_multi_job(&job.id, &job.skills, singles, job.places.pickups.len());
            job_index.insert(job.id.clone(), problem_job.clone());
            jobs.push(problem_job)
        }
    });

    (jobs, vec![])
}

fn read_conditional_jobs(
    api_problem: &ApiProblem,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> (Vec<Arc<Job>>, Vec<Arc<Lock>>) {
    let mut jobs = vec![];
    api_problem.fleet.types.iter().for_each(|vehicle| {
        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            if let Some(breaks) = &shift.breaks {
                read_breaks(coord_index, job_index, &mut jobs, vehicle, shift_index, breaks);
            }

            if let Some(max_tour) = shift.max_tours {
                // NOTE actually, we are not limited by start/end places
                let location = Some(shift.start.location.clone());
                // TODO read value from problem
                let duration = 15. * 60.;
                let times = Some(vec![vec![
                    shift.start.time.clone(),
                    shift
                        .end
                        .as_ref()
                        .and_then(|p| Some(p.time.clone()))
                        .unwrap_or_else(|| "2970-01-01T00:00:00Z".to_string()),
                ]]);
                let reloads = (1..max_tour).fold(vec![], |mut acc, _| {
                    acc.push((&location, duration, &times));
                    acc
                });
                read_reloads(coord_index, job_index, &mut jobs, vehicle, shift_index, reloads);
            }
        }
    });

    (jobs, vec![])
}

fn read_breaks(
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
    jobs: &mut Vec<Arc<Job>>,
    vehicle: &VehicleType,
    shift_index: usize,
    breaks: &Vec<VehicleBreak>,
) {
    (1..).zip(breaks.iter()).for_each(|(break_idx, place)| {
        (1..vehicle.amount + 1).for_each(|index| {
            let times = if place.times.is_empty() {
                panic!("Break without any time window does not make sense!")
            } else {
                Some(place.times.clone())
            };
            add_conditional_job(
                coord_index,
                job_index,
                jobs,
                format!("{}_{}", vehicle.id, index),
                "break",
                shift_index,
                break_idx,
                (&place.location, place.duration, &times),
            );
        });
    });
}

fn read_reloads(
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
    jobs: &mut Vec<Arc<Job>>,
    vehicle: &VehicleType,
    shift_index: usize,
    reloads: Vec<(&Option<Vec<f64>>, Duration, &Option<Vec<Vec<String>>>)>,
) {
    (1..).zip(reloads.iter()).for_each(|(reload_idx, (location, duration, times))| {
        (1..vehicle.amount + 1).for_each(|index| {
            let vehicle_id = format!("{}_{}", vehicle.id, index);
            add_conditional_job(
                coord_index,
                job_index,
                jobs,
                vehicle_id.clone(),
                "reload",
                shift_index,
                reload_idx,
                (location, *duration, times),
            );
        });
    });
}

fn add_conditional_job(
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
    jobs: &mut Vec<Arc<Job>>,
    vehicle_id: String,
    job_type: &str,
    shift_index: usize,
    index: usize,
    place: (&Option<Vec<f64>>, Duration, &Option<Vec<Vec<String>>>),
) {
    let (location, duration, times) = place;
    let mut single = get_single(location.as_ref().and_then(|l| Some(l)), duration, &times, coord_index);
    single.dimens.set_id(job_type);
    single.dimens.insert("type".to_string(), Box::new(job_type.to_string()));
    single.dimens.insert("shift_index".to_string(), Box::new(shift_index));
    single.dimens.insert("vehicle_id".to_string(), Box::new(vehicle_id.clone()));

    let job = Arc::new(Job::Single(Arc::new(single)));
    job_index.insert(format!("{}_{}_{}", vehicle_id, job_type, index), job.clone());
    jobs.push(job);
}

fn get_single(
    location: Option<&Vec<f64>>,
    duration: Duration,
    times: &Option<Vec<Vec<String>>>,
    coord_index: &CoordIndex,
) -> Single {
    Single {
        places: vec![Place {
            location: location.as_ref().and_then(|l| coord_index.get_by_vec(l)),
            duration,
            times: times
                .as_ref()
                .map_or(vec![TimeWindow::max()], |tws| tws.iter().map(|tw| parse_time_window(tw)).collect()),
        }],
        dimens: Default::default(),
    }
}

fn get_single_with_extras(
    location: &Vec<f64>,
    duration: Duration,
    demand: Demand<MultiDimensionalCapacity>,
    times: &Option<Vec<Vec<String>>>,
    tag: &Option<String>,
    activity_type: &str,
    has_multi_dimens: bool,
    coord_index: &CoordIndex,
) -> Single {
    let mut single = get_single(Some(location), duration, times, coord_index);
    if has_multi_dimens {
        single.dimens.set_demand(demand);
    } else {
        single.dimens.set_demand(Demand {
            pickup: (demand.pickup.0.capacity[0], demand.pickup.1.capacity[0]),
            delivery: (demand.delivery.0.capacity[0], demand.delivery.1.capacity[0]),
        });
    }
    single.dimens.insert("type".to_string(), Box::new(activity_type.to_string()));
    add_tag(&mut single.dimens, tag);

    single
}

fn get_single_job(id: &String, single: Single, skills: &Option<Vec<String>>) -> Arc<Job> {
    let mut single = single;
    single.dimens.set_id(id.as_str());
    add_skills(&mut single.dimens, skills);

    Arc::new(Job::Single(Arc::new(single)))
}

fn get_multi_job(
    id: &String,
    skills: &Option<Vec<String>>,
    singles: Vec<Arc<Single>>,
    deliveries_start_index: usize,
) -> Arc<Job> {
    let mut dimens: Dimensions = Default::default();
    dimens.set_id(id.as_str());
    add_skills(&mut dimens, skills);
    let multi = if singles.len() == 2 && deliveries_start_index == 1 {
        Multi::new(singles, dimens)
    } else {
        Multi::new_with_generator(
            singles,
            dimens,
            Box::new(move |m| get_split_permutations(m.jobs.len(), deliveries_start_index, MULTI_JOB_SAMPLE_SIZE)),
        )
    };
    Arc::new(Job::Multi(Multi::bind(multi)))
}

fn create_condition(vehicle_id: String, shift_index: usize) -> Arc<dyn Fn(&Actor) -> bool + Sync + Send> {
    Arc::new(move |actor: &Actor| {
        *actor.vehicle.dimens.get_id().unwrap() == vehicle_id
            && *actor.vehicle.dimens.get_value::<usize>("shift_index").unwrap() == shift_index
    })
}

fn add_tag(dimens: &mut Dimensions, tag: &Option<String>) {
    if let Some(tag) = tag {
        dimens.insert("tag".to_string(), Box::new(tag.clone()));
    }
}

fn empty() -> MultiDimensionalCapacity {
    MultiDimensionalCapacity::default()
}
