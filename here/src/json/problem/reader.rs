#[cfg(test)]
#[path = "../../../tests/unit/json/problem/reader_test.rs"]
mod reader_test;

use super::StringReader;
use crate::constraints::BreakModule;
use crate::json::coord_index::CoordIndex;
use crate::json::problem::{deserialize_matrix, deserialize_problem, JobVariant, Matrix, RelationType};
use chrono::DateTime;
use core::construction::constraints::*;
use core::models::common::*;
use core::models::problem::*;
use core::models::{Extras, Lock, LockDetail, LockOrder, LockPosition, Problem};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

type ApiProblem = crate::json::problem::Problem;
type JobIndex = HashMap<String, Arc<Job>>;

/// Reads specific problem definition from various sources.
pub trait HereProblem {
    fn read_here(self) -> Result<Problem, String>;
}

impl HereProblem for (File, Vec<File>) {
    fn read_here(self) -> Result<Problem, String> {
        // TODO consume files and close them?
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

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, String> {
    let coord_index = create_coord_index(&api_problem);
    let transport = Arc::new(create_transport_costs(&matrices));
    let activity = Arc::new(SimpleActivityCost::default());
    let fleet = read_fleet(&api_problem, &coord_index);

    let mut job_index = Default::default();
    let jobs = read_jobs(&api_problem, &coord_index, &fleet, transport.as_ref(), &mut job_index);
    let locks = read_locks(&api_problem, &job_index);
    let limits = read_limits(&api_problem);

    let constraint = create_constraint_pipeline(
        &fleet,
        activity.clone(),
        transport.clone(),
        locks.iter().cloned().collect(),
        limits,
    );
    let extras = create_extras(&api_problem.id, coord_index);

    Ok(Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks,
        constraint: Arc::new(constraint),
        activity,
        transport,
        extras: Arc::new(extras),
    })
}

fn create_coord_index(api_problem: &ApiProblem) -> CoordIndex {
    let mut index = CoordIndex::new();

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
        index.add_from_vec(&vehicle.places.start.location);

        if let Some(end) = &vehicle.places.end {
            index.add_from_vec(&end.location);
        }

        if let Some(vehicle_break) = &vehicle.vehicle_break {
            if let Some(location) = &vehicle_break.location {
                index.add_from_vec(location);
            }
        }
    });

    index
}

fn create_transport_costs(matrices: &Vec<Matrix>) -> MatrixTransportCost {
    let mut durations: Vec<Vec<Duration>> = Default::default();
    let mut distances: Vec<Vec<Distance>> = Default::default();

    matrices.iter().for_each(|matrix| {
        // TODO process error codes
        durations.push(matrix.travel_times.iter().map(|d| *d as f64).collect());
        distances.push(matrix.distances.iter().map(|d| *d as f64).collect());
    });

    MatrixTransportCost::new(durations, distances)
}

fn read_fleet(api_problem: &ApiProblem, coord_index: &CoordIndex) -> Fleet {
    let profiles = get_profile_map(api_problem);
    let mut vehicles: Vec<Vehicle> = Default::default();

    api_problem.fleet.types.iter().for_each(|vehicle| {
        // TODO support multi-dimensional capacity
        assert_eq!(vehicle.capacity.len(), 1);

        let start = {
            let location = coord_index.get_by_vec(&vehicle.places.start.location).unwrap();
            let time = parse_time(&vehicle.places.start.time);
            (location, time)
        };

        let end = vehicle.places.end.as_ref().map_or(None, |end| {
            let location = coord_index.get_by_vec(&end.location).unwrap();
            let time = parse_time(&end.time);
            Some((location, time))
        });

        let details = vec![VehicleDetail {
            start: Some(start.0),
            end: end.map_or(None, |end| Some(end.0)),
            time: Some(TimeWindow::new(start.1, end.map_or(std::f64::MAX, |end| end.1))),
        }];

        let costs = Costs {
            fixed: vehicle.costs.fixed.unwrap_or(0.),
            per_distance: vehicle.costs.distance,
            per_driving_time: vehicle.costs.time,
            per_waiting_time: vehicle.costs.time,
            per_service_time: vehicle.costs.time,
        };

        let profile = *profiles.get(&vehicle.profile).unwrap() as Profile;

        (1..vehicle.amount + 1).for_each(|number| {
            let mut dimens: Dimensions = Default::default();
            dimens.insert("type_id".to_owned(), Box::new(vehicle.id.clone()));
            dimens.set_id(format!("{}_{}", vehicle.id, number.to_string()).as_str());
            dimens.set_capacity(*vehicle.capacity.first().unwrap());
            add_skills(&mut dimens, &vehicle.skills);

            vehicles.push(Vehicle { profile, costs: costs.clone(), dimens, details: details.clone() });
        });
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
    locks: Vec<Arc<Lock>>,
    limit_func: Option<TravelLimitFunc>,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport.clone(), 1)));
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));
    constraint.add_module(Box::new(BreakModule::new(4)));

    if !locks.is_empty() {
        constraint.add_module(Box::new(StrictLockingModule::new(fleet, locks, 3)));
    }

    if let Some(limit_func) = limit_func {
        constraint.add_module(Box::new(TravelModule::new(limit_func.clone(), transport.clone(), 5, 6)));
    }

    constraint
}

fn create_extras(problem_id: &String, coord_index: CoordIndex) -> Extras {
    let mut extras = Extras::default();
    extras.insert("coord_index".to_owned(), Box::new(coord_index));
    extras.insert("problem_id".to_string(), Box::new(problem_id.clone()));

    extras
}

fn read_jobs(
    api_problem: &ApiProblem,
    coord_index: &CoordIndex,
    fleet: &Fleet,
    transport: &impl TransportCost,
    job_index: &mut JobIndex,
) -> Jobs {
    let mut jobs = read_required_jobs(api_problem, coord_index, job_index);
    jobs.extend(read_conditional_jobs(api_problem, coord_index, job_index));

    Jobs::new(fleet, jobs, transport)
}

fn read_required_jobs(api_problem: &ApiProblem, coord_index: &CoordIndex, job_index: &mut JobIndex) -> Vec<Arc<Job>> {
    let mut jobs = vec![];
    api_problem.plan.jobs.iter().for_each(|job| match job {
        JobVariant::Single(job) => {
            let demand = *job.demand.first().unwrap();
            let is_shipment = job.places.pickup.is_some() && job.places.delivery.is_some();
            let demand = if is_shipment { (0, demand) } else { (demand, 0) };

            let pickup = job.places.pickup.as_ref().map(|pickup| {
                get_single_with_extras(
                    &pickup.location,
                    pickup.duration,
                    Demand { pickup: demand.clone(), delivery: (0, 0) },
                    &pickup.times,
                    &pickup.tag,
                    "pickup",
                    &coord_index,
                )
            });
            let delivery = job.places.delivery.as_ref().map(|delivery| {
                get_single_with_extras(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (0, 0), delivery: demand },
                    &delivery.times,
                    &delivery.tag,
                    "delivery",
                    &coord_index,
                )
            });

            let problem_job = match (pickup, delivery) {
                (Some(pickup), Some(delivery)) => {
                    get_multi_job(&job.id, &job.skills, vec![Arc::new(pickup), Arc::new(delivery)])
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
                    let demand = *pickup.demand.first().unwrap();
                    Arc::new(get_single_with_extras(
                        &pickup.location,
                        pickup.duration,
                        Demand { pickup: (0, demand), delivery: (0, 0) },
                        &pickup.times,
                        &pickup.tag,
                        "pickup",
                        &coord_index,
                    ))
                })
                .collect::<Vec<Arc<Single>>>();
            singles.extend(job.places.deliveries.iter().map(|delivery| {
                let demand = *delivery.demand.first().unwrap();
                Arc::new(get_single_with_extras(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (0, 0), delivery: (0, demand) },
                    &delivery.times,
                    &delivery.tag,
                    "delivery",
                    &coord_index,
                ))
            }));

            let problem_job = get_multi_job(&job.id, &job.skills, singles);
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
    api_problem.fleet.types.iter().filter(|v| v.vehicle_break.is_some()).for_each(|vehicle| {
        let place = vehicle.vehicle_break.as_ref().unwrap();
        (1..vehicle.amount + 1).for_each(|index| {
            let id = format!("{}_{}", vehicle.id, index);
            let times = if place.times.is_empty() {
                panic!("Break without any time window does not make sense!")
            } else {
                Some(place.times.clone())
            };
            let mut single =
                get_single(place.location.as_ref().and_then(|l| Some(l)), place.duration, &times, coord_index);
            single.dimens.set_id("break");
            single.dimens.insert("type".to_string(), Box::new("break".to_string()));
            single.dimens.insert("vehicle_id".to_string(), Box::new(id.clone()));

            let job = Arc::new(Job::Single(Arc::new(single)));
            job_index.insert(format!("{}_break", id), job.clone());
            jobs.push(job);
        });
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
            if let Some(limits) = limits.get(actor.vehicle.dimens.get_id().unwrap()) {
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
    demand: Demand<i32>,
    times: &Option<Vec<Vec<String>>>,
    tag: &Option<String>,
    activity_type: &str,
    coord_index: &CoordIndex,
) -> Single {
    let mut single = get_single(Some(location), duration, times, coord_index);
    single.dimens.set_demand(demand);
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

fn get_multi_job(id: &String, skills: &Option<Vec<String>>, singles: Vec<Arc<Single>>) -> Arc<Job> {
    let mut dimens: Dimensions = Default::default();
    dimens.set_id(id.as_str());
    add_skills(&mut dimens, skills);
    let multi = Multi::new(singles, dimens);
    Arc::new(Job::Multi(Multi::bind(multi)))
}

fn add_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.insert("skills".to_owned(), Box::new(skills.clone()));
    }
}

fn add_tag(dimens: &mut Dimensions, tag: &Option<String>) {
    if let Some(tag) = tag {
        dimens.insert("tag".to_string(), Box::new(tag.clone()));
    }
}

// endregion
