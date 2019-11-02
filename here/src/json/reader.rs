use crate::json::coord_index::CoordIndex;
use chrono::DateTime;
use core::construction::constraints::{CapacityDimension, Demand, DemandDimension};
use core::models::common::*;
use core::models::problem::*;
use core::models::{Lock, Problem};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::slice::Iter;
use std::sync::Arc;

#[path = "./deserializer.rs"]
mod deserializer;

use self::deserializer::{deserialize_matrix, deserialize_problem, JobVariant, Matrix};
use crate::json::reader::deserializer::JobPlace;

type ApiProblem = self::deserializer::Problem;
type JobIndex = HashMap<String, Arc<Job>>;

/// Reads specific problem definition from various sources.
pub trait HereProblem {
    fn parse_here(&self) -> Result<Problem, String>;
}

impl HereProblem for (File, Vec<File>) {
    fn parse_here(&self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(&self.0)).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(matrix)).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (String, Vec<String>) {
    fn parse_here(&self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(StringReader::new(&self.0))).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(StringReader::new(matrix))).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, String> {
    let coord_index = create_coord_index(&api_problem);
    let transport_costs = create_transport_costs(&matrices);
    let fleet = read_fleet(&api_problem, &coord_index);

    let mut job_index = Default::default();
    let jobs = read_jobs(&api_problem, &coord_index, &fleet, &transport_costs, &mut job_index);
    let locks = read_locks(&api_problem, &jobs, &job_index);
    let limits = read_limits(&api_problem);

    unimplemented!()
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

    (0..).zip(matrices.iter()).for_each(|(index, matrix)| {
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

        (1..vehicle.amount).for_each(|number| {
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
    api_problem.plan.jobs.iter().for_each(|job| match job {
        JobVariant::Single(job) => {
            let demand = *job.demand.first().unwrap();
            let pickup = job.places.pickup.as_ref().map(|pickup| {
                get_single(
                    &pickup.location,
                    pickup.duration,
                    Demand { pickup: (demand, 0), delivery: (0, 0) },
                    &pickup.times,
                    &pickup.tag,
                    &coord_index,
                )
            });
            let delivery = job.places.delivery.as_ref().map(|delivery| {
                get_single(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (0, 0), delivery: (demand, 0) },
                    &delivery.times,
                    &delivery.tag,
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

            job_index.insert(job.id.clone(), problem_job);
        }
        JobVariant::Multi(job) => {
            let mut singles = job
                .places
                .pickups
                .iter()
                .map(|pickup| {
                    let demand = *pickup.demand.first().unwrap();
                    Arc::new(get_single(
                        &pickup.location,
                        pickup.duration,
                        Demand { pickup: (0, demand), delivery: (0, 0) },
                        &pickup.times,
                        &pickup.tag,
                        &coord_index,
                    ))
                })
                .collect::<Vec<Arc<Single>>>();
            singles.extend(job.places.deliveries.iter().map(|delivery| {
                let demand = *delivery.demand.first().unwrap();
                Arc::new(get_single(
                    &delivery.location,
                    delivery.duration,
                    Demand { pickup: (0, 0), delivery: (0, demand) },
                    &delivery.times,
                    &delivery.tag,
                    &coord_index,
                ))
            }));

            job_index.insert(job.id.clone(), get_multi_job(&job.id, &job.skills, singles));
        }
    });

    unimplemented!()
}

fn read_conditional_jobs(
    api_problem: &ApiProblem,
    coord_index: &CoordIndex,
    job_index: &mut JobIndex,
) -> Vec<Arc<Job>> {
    unimplemented!()
}

fn read_locks(api_problem: &ApiProblem, jobs: &Jobs, job_index: &JobIndex) -> Option<Vec<Lock>> {
    unimplemented!()
}

fn read_limits(
    api_problem: &ApiProblem,
) -> Option<Arc<dyn Fn(&Arc<Actor>) -> (Option<Distance>, Option<Duration>) + Send + Sync>> {
    unimplemented!()
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
        if !acc.get(&vehicle.profile).is_none() {
            acc.insert(vehicle.profile.clone(), acc.len());
        }
        acc
    })
}

// region helpers

fn get_single(
    location: &Vec<f64>,
    duration: Duration,
    demand: Demand<i32>,
    times: &Option<Vec<Vec<String>>>,
    tag: &Option<String>,
    coord_index: &CoordIndex,
) -> Single {
    let mut dimens: Dimensions = Default::default();
    dimens.set_demand(demand);
    add_tag(&mut dimens, tag);

    Single {
        places: vec![Place {
            location: coord_index.get_by_vec(location),
            duration,
            times: times
                .as_ref()
                .map_or(vec![TimeWindow::max()], |tws| tws.iter().map(|tw| parse_time_window(tw)).collect()),
        }],
        dimens,
    }
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

// region utils

struct StringReader<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { iter: data.as_bytes().iter() }
    }
}

impl<'a> Read for StringReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for i in 0..buf.len() {
            if let Some(x) = self.iter.next() {
                buf[i] = *x;
            } else {
                return Ok(i);
            }
        }
        Ok(buf.len())
    }
}

// endregion
