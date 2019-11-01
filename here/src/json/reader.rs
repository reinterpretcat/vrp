use crate::json::coord_index::CoordIndex;
use chrono::DateTime;
use core::construction::constraints::CapacityDimension;
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
            let location = *coord_index.get_by_vec(&vehicle.places.start.location).unwrap();
            let time = parse_time(&vehicle.places.start.time);
            (location, time)
        };

        let end = vehicle.places.end.as_ref().map_or(None, |end| {
            let location = *coord_index.get_by_vec(&end.location).unwrap();
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
            if let Some(skills) = &vehicle.skills {
                dimens.insert("skills".to_owned(), Box::new(skills.clone()));
            }

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

fn get_profile_map(api_problem: &ApiProblem) -> HashMap<String, usize> {
    api_problem.fleet.types.iter().fold(Default::default(), |mut acc, vehicle| {
        if !acc.get(&vehicle.profile).is_none() {
            acc.insert(vehicle.profile.clone(), acc.len());
        }
        acc
    })
}

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
