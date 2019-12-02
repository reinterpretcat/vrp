#[cfg(test)]
#[path = "../../../tests/unit/json/problem/reader_test.rs"]
mod reader_test;

use super::StringReader;
use crate::constraints::{BreakModule, ExtraCostModule, ReachableModule, SkillsModule};
use crate::extensions::{MultiDimensionalCapacity, OnlyVehicleActivityCost};
use crate::json::coord_index::CoordIndex;
use crate::json::problem::{deserialize_matrix, deserialize_problem, JobVariant, Matrix, RelationType};
use crate::utils::get_split_permutations;
use chrono::DateTime;
use core::construction::constraints::*;
use core::models::common::*;
use core::models::problem::*;
use core::models::{Extras, Lock, LockDetail, LockOrder, LockPosition, Problem};
use core::refinement::objectives::PenalizeUnassigned;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::sync::Arc;

// TODO configure sample size
const MULTI_JOB_SAMPLE_SIZE: usize = 3;

type ApiProblem = crate::json::problem::Problem;
type JobIndex = HashMap<String, Arc<Job>>;

/// Reads specific problem definition from various sources.
pub trait HereProblem {
    fn read_here(self) -> Result<Problem, String>;
}

impl HereProblem for (File, Vec<File>) {
    fn read_here(self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(&self.0)).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(matrix)).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (String, Vec<String>) {
    fn read_here(self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(StringReader::new(&self.0))).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(StringReader::new(matrix))).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (ApiProblem, Vec<Matrix>) {
    fn read_here(self) -> Result<Problem, String> {
        map_to_problem(self.0, self.1)
    }
}

struct ProblemProperties {
    has_multi_dimen_capacity: bool,
    has_breaks: bool,
    has_skills: bool,
    has_unreachable_locations: bool,
    has_fixed_cost: bool,
}

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, String> {
    let problem_props = get_problem_properties(&api_problem, &matrices);

    let coord_index = create_coord_index(&api_problem);
    let transport = Arc::new(create_transport_costs(&matrices));
    let activity = Arc::new(OnlyVehicleActivityCost::default());
    let fleet = read_fleet(&api_problem, &problem_props, &coord_index);

    let mut job_index = Default::default();
    let jobs = read_jobs(&api_problem, &problem_props, &coord_index, &fleet, transport.as_ref(), &mut job_index);
    let locks = read_locks(&api_problem, &job_index);
    let limits = read_limits(&api_problem);
    let extras = create_extras(&api_problem.id, &problem_props, coord_index);
    let constraint =
        create_constraint_pipeline(&fleet, activity.clone(), transport.clone(), problem_props, &locks, limits);

    Ok(Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks,
        constraint: Arc::new(constraint),
        activity,
        transport,
        objective: Arc::new(PenalizeUnassigned::default()),
        extras: Arc::new(extras),
    })
}

fn create_coord_index(api_problem: &ApiProblem) -> CoordIndex {
    let mut index = CoordIndex::default();

    // process plan
    api_problem.plan.jobs.iter().for_each(|job| match &job {
        JobVariant::Single(job) => {
            if let Some(pickup) = &job.places.pickup {
                index.add_from_vec(&pickup.location);
            }
            if let Some(delivery) = &job.places.delivery {
                index.add_from_vec(&delivery.location);
            }
        }
        JobVariant::Multi(job) => {
            job.places.pickups.iter().for_each(|pickup| {
                index.add_from_vec(&pickup.location);
            });
            job.places.deliveries.iter().for_each(|delivery| {
                index.add_from_vec(&delivery.location);
            });
        }
    });

    // process fleet
    api_problem.fleet.types.iter().for_each(|vehicle| {
        vehicle.shifts.iter().for_each(|shift| {
            index.add_from_vec(&shift.start.location);

            if let Some(end) = &shift.end {
                index.add_from_vec(&end.location);
            }

            if let Some(breaks) = &shift.breaks {
                breaks.iter().for_each(|vehicle_break| {
                    if let Some(location) = &vehicle_break.location {
                        index.add_from_vec(location);
                    }
                });
            }
        });
    });

    index
}

fn create_transport_costs(matrices: &Vec<Matrix>) -> MatrixTransportCost {
    let mut all_durations: Vec<Vec<Duration>> = Default::default();
    let mut all_distances: Vec<Vec<Distance>> = Default::default();

    matrices.iter().for_each(|matrix| {
        if let Some(error_codes) = &matrix.error_codes {
            let mut profile_durations: Vec<Duration> = Default::default();
            let mut profile_distances: Vec<Distance> = Default::default();
            for (i, error) in error_codes.iter().enumerate() {
                if *error > 0 {
                    profile_durations.push(-1.);
                    profile_distances.push(-1.);
                } else {
                    profile_durations.push(*matrix.travel_times.get(i).unwrap() as f64);
                    profile_distances.push(*matrix.distances.get(i).unwrap() as f64);
                }
            }
            all_durations.push(profile_durations);
            all_distances.push(profile_distances);
        } else {
            all_durations.push(matrix.travel_times.iter().map(|d| *d as f64).collect());
            all_distances.push(matrix.distances.iter().map(|d| *d as f64).collect());
        }
    });

    MatrixTransportCost::new(all_durations, all_distances)
}

fn read_fleet(api_problem: &ApiProblem, props: &ProblemProperties, coord_index: &CoordIndex) -> Fleet {
    let profiles = get_profile_map(api_problem);
    let mut vehicles: Vec<Vehicle> = Default::default();

    api_problem.fleet.types.iter().for_each(|vehicle| {
        let costs = Costs {
            fixed: vehicle.costs.fixed.unwrap_or(0.),
            per_distance: vehicle.costs.distance,
            per_driving_time: vehicle.costs.time,
            per_waiting_time: vehicle.costs.time,
            per_service_time: vehicle.costs.time,
        };

        let profile = *profiles.get(&vehicle.profile).unwrap() as Profile;

        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            let start = {
                let location = coord_index.get_by_vec(&shift.start.location).unwrap();
                let time = parse_time(&shift.start.time);
                (location, time)
            };

            let end = shift.end.as_ref().map_or(None, |end| {
                let location = coord_index.get_by_vec(&end.location).unwrap();
                let time = parse_time(&end.time);
                Some((location, time))
            });

            let details = vec![VehicleDetail {
                start: Some(start.0),
                end: end.map_or(None, |end| Some(end.0)),
                time: Some(TimeWindow::new(start.1, end.map_or(std::f64::MAX, |end| end.1))),
            }];

            (1..vehicle.amount + 1).for_each(|number| {
                let mut dimens: Dimensions = Default::default();
                dimens.insert("type_id".to_owned(), Box::new(vehicle.id.clone()));
                dimens.insert("shift_index".to_owned(), Box::new(shift_index));
                dimens.set_id(format!("{}_{}", vehicle.id, number.to_string()).as_str());

                if props.has_multi_dimen_capacity {
                    dimens.set_capacity(MultiDimensionalCapacity::new(vehicle.capacity.clone()));
                } else {
                    dimens.set_capacity(*vehicle.capacity.first().unwrap());
                }
                add_skills(&mut dimens, &vehicle.skills);

                vehicles.push(Vehicle { profile, costs: costs.clone(), dimens, details: details.clone() });
            });
        }
    });

    let fake_driver = Driver {
        costs: Costs {
            fixed: 0.0,
            per_distance: 0.0,
            per_driving_time: 0.0,
            per_waiting_time: 0.0,
            per_service_time: 0.0,
        },
        dimens: Default::default(),
        details: vec![],
    };

    Fleet::new(vec![fake_driver], vehicles)
}

fn create_constraint_pipeline(
    fleet: &Fleet,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    props: ProblemProperties,
    locks: &Vec<Arc<Lock>>,
    limits: Option<TravelLimitFunc>,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport.clone(), 1)));

    if props.has_multi_dimen_capacity {
        constraint.add_module(Box::new(CapacityConstraintModule::<MultiDimensionalCapacity>::new(2)));
    } else {
        constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));
    }

    if props.has_breaks {
        constraint.add_module(Box::new(BreakModule::new(4, Some(-100.), false)));
    }

    if props.has_skills {
        constraint.add_module(Box::new(SkillsModule::new(10)));
    }

    if !locks.is_empty() {
        constraint.add_module(Box::new(StrictLockingModule::new(fleet, locks.clone(), 3)));
    }

    if let Some(limits) = limits {
        constraint.add_module(Box::new(TravelModule::new(limits.clone(), transport.clone(), 5, 6)));
    }

    if props.has_unreachable_locations {
        constraint.add_module(Box::new(ReachableModule::new(transport.clone(), 11)));
    }

    if props.has_fixed_cost {
        constraint.add_module(Box::new(ExtraCostModule::default()));
    }

    constraint
}

fn create_extras(problem_id: &String, props: &ProblemProperties, coord_index: CoordIndex) -> Extras {
    let mut extras = Extras::default();
    extras.insert("problem_id".to_string(), Box::new(problem_id.clone()));
    extras.insert(
        "capacity_type".to_string(),
        Box::new((if props.has_multi_dimen_capacity { "multi" } else { "single" }).to_string()),
    );
    extras.insert("coord_index".to_owned(), Box::new(coord_index));

    extras
}

fn read_jobs(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    fleet: &Fleet,
    transport: &impl TransportCost,
    job_index: &mut JobIndex,
) -> Jobs {
    let mut jobs = read_required_jobs(api_problem, props, coord_index, job_index);
    jobs.extend(read_conditional_jobs(api_problem, coord_index, job_index));

    Jobs::new(fleet, jobs, transport)
}

fn read_required_jobs(
    api_problem: &ApiProblem,
    props: &ProblemProperties,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> Vec<Arc<Job>> {
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

    jobs
}

fn read_conditional_jobs(
    api_problem: &ApiProblem,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> Vec<Arc<Job>> {
    let mut jobs = vec![];

    api_problem.fleet.types.iter().for_each(|vehicle| {
        for (shift_index, shift) in vehicle.shifts.iter().enumerate() {
            if let Some(breaks) = &shift.breaks {
                breaks.iter().for_each(|place| {
                    (1..vehicle.amount + 1).for_each(|index| {
                        let id = format!("{}_{}", vehicle.id, index);
                        let times = if place.times.is_empty() {
                            panic!("Break without any time window does not make sense!")
                        } else {
                            Some(place.times.clone())
                        };
                        let mut single = get_single(
                            place.location.as_ref().and_then(|l| Some(l)),
                            place.duration,
                            &times,
                            coord_index,
                        );
                        single.dimens.set_id("break");
                        single.dimens.insert("type".to_string(), Box::new("break".to_string()));
                        single.dimens.insert("shift_index".to_string(), Box::new(shift_index));
                        single.dimens.insert("vehicle_id".to_string(), Box::new(id.clone()));

                        let job = Arc::new(Job::Single(Arc::new(single)));
                        job_index.insert(format!("{}_break", id), job.clone());
                        jobs.push(job);
                    });
                });
            }
        }
    });

    jobs
}

fn read_locks(api_problem: &ApiProblem, job_index: &JobIndex) -> Vec<Arc<Lock>> {
    if api_problem.plan.relations.as_ref().map_or(true, |r| r.is_empty()) {
        return vec![];
    }

    let relations = api_problem.plan.relations.as_ref().unwrap().iter().fold(HashMap::new(), |mut acc, r| {
        acc.entry(r.vehicle_id.clone()).or_insert(vec![]).push(r.clone());
        acc
    });

    let locks = relations.into_iter().fold(vec![], |mut acc, (vehicle_id, rels)| {
        let vehicle_id_copy = vehicle_id.clone();
        let condition = Arc::new(move |a: &Actor| *a.vehicle.dimens.get_id().unwrap() == vehicle_id_copy);
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

            let jobs = rel
                .jobs
                .iter()
                .filter(|job| job.as_str() != "departure" && job.as_str() != "arrival")
                .map(|job| {
                    if job.as_str() == "break" {
                        job_index.get(format!("{}_break", vehicle_id).as_str()).unwrap().clone()
                    } else {
                        job_index.get(job).unwrap().clone()
                    }
                })
                .collect();

            acc.push(LockDetail::new(order, position, jobs));

            acc
        });

        acc.push(Arc::new(Lock::new(condition, details)));

        acc
    });

    locks
}

fn read_limits(api_problem: &ApiProblem) -> Option<TravelLimitFunc> {
    let limits = api_problem.fleet.types.iter().filter(|vehicle| vehicle.limits.is_some()).fold(
        HashMap::new(),
        |mut acc, vehicle| {
            let limits = vehicle.limits.as_ref().unwrap().clone();
            acc.insert(vehicle.id.clone(), (limits.max_distance, limits.shift_time));
            acc
        },
    );

    if limits.is_empty() {
        None
    } else {
        Some(Arc::new(move |actor: &Actor| {
            if let Some(limits) = limits.get(actor.vehicle.dimens.get_value::<String>("type_id").unwrap()) {
                (limits.0, limits.1)
            } else {
                (None, None)
            }
        }))
    }
}

fn parse_time(time: &String) -> Timestamp {
    let time = DateTime::parse_from_rfc3339(time).unwrap();
    time.timestamp() as Timestamp
}

fn parse_time_window(tw: &Vec<String>) -> TimeWindow {
    assert_eq!(tw.len(), 2);
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_profile_map(api_problem: &ApiProblem) -> HashMap<String, usize> {
    api_problem.fleet.types.iter().fold(Default::default(), |mut acc, vehicle| {
        if acc.get(&vehicle.profile) == None {
            acc.insert(vehicle.profile.clone(), acc.len());
        }
        acc
    })
}

fn get_problem_properties(api_problem: &ApiProblem, matrices: &Vec<Matrix>) -> ProblemProperties {
    let has_unreachable_locations = matrices.iter().any(|m| m.error_codes.is_some());
    let has_multi_dimen_capacity = api_problem.fleet.types.iter().any(|t| t.capacity.len() > 1)
        || api_problem.plan.jobs.iter().any(|j| match j {
            JobVariant::Single(job) => job.demand.len() > 1,
            JobVariant::Multi(job) => {
                job.places.pickups.iter().any(|p| p.demand.len() > 1)
                    || job.places.deliveries.iter().any(|p| p.demand.len() > 1)
            }
        });
    let has_breaks = api_problem
        .fleet
        .types
        .iter()
        .flat_map(|t| &t.shifts)
        .any(|shift| shift.breaks.as_ref().map_or(false, |b| b.len() > 0));
    let has_skills = api_problem.plan.jobs.iter().any(|j| match j {
        JobVariant::Single(job) => job.skills.is_some(),
        JobVariant::Multi(job) => job.skills.is_some(),
    });
    let has_fixed_cost = api_problem.fleet.types.iter().any(|t| t.costs.fixed.is_some());

    ProblemProperties { has_multi_dimen_capacity, has_breaks, has_skills, has_unreachable_locations, has_fixed_cost }
}

// region helpers

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

fn add_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.insert("skills".to_owned(), Box::new(HashSet::<String>::from_iter(skills.iter().cloned())));
    }
}

fn add_tag(dimens: &mut Dimensions, tag: &Option<String>) {
    if let Some(tag) = tag {
        dimens.insert("tag".to_string(), Box::new(tag.clone()));
    }
}

fn empty() -> MultiDimensionalCapacity {
    MultiDimensionalCapacity::default()
}

// endregion
