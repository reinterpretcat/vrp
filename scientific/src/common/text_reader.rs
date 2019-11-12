#[cfg(test)]
#[path = "../../tests/unit/common/text_reader_test.rs"]
mod text_reader_test;

use core::construction::constraints::*;
use core::construction::states::{create_end_activity, create_start_activity};
use core::models::common::*;
use core::models::problem::*;
use core::models::solution::{Activity, Registry, Route, Tour};
use core::models::{Problem, Solution};
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{BufReader, Read};
use std::slice::Iter;
use std::sync::Arc;

pub struct StringReader<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { iter: data.as_bytes().iter() }
    }
}

impl<'a> Read for StringReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for (i, item) in buf.iter_mut().enumerate() {
            if let Some(x) = self.iter.next() {
                *item = *x;
            } else {
                return Ok(i);
            }
        }
        Ok(buf.len())
    }
}

pub trait TextReader {
    fn read_problem(&mut self) -> Result<Problem, String> {
        let fleet = self.read_fleet()?;
        let jobs = self.read_jobs()?;
        let transport = Arc::new(self.create_transport());
        let activity = Arc::new(SimpleActivityCost::default());
        let jobs = Jobs::new(&fleet, jobs, transport.as_ref());

        Ok(Problem {
            fleet: Arc::new(fleet),
            jobs: Arc::new(jobs),
            locks: vec![],
            constraint: Arc::new(create_constraint(activity.clone(), transport.clone())),
            activity,
            transport,
            extras: Arc::new(Default::default()),
        })
    }

    fn read_fleet(&mut self) -> Result<Fleet, String>;

    fn read_jobs(&mut self) -> Result<Vec<Arc<Job>>, String>;

    fn create_transport(&self) -> MatrixTransportCost;
}

pub fn create_fleet_with_distance_costs(number: usize, capacity: usize, location: Location, time: TimeWindow) -> Fleet {
    Fleet::new(
        vec![Driver {
            costs: Costs {
                fixed: 0.0,
                per_distance: 0.0,
                per_driving_time: 0.0,
                per_waiting_time: 0.0,
                per_service_time: 0.0,
            },
            dimens: create_dimens_with_id("driver", 0),
            details: Default::default(),
        }],
        (0..number)
            .map(|i| {
                let mut dimens = create_dimens_with_id("v", i);
                dimens.set_capacity(capacity as i32);
                Vehicle {
                    profile: 0,
                    costs: Costs {
                        fixed: 0.0,
                        per_distance: 1.0,
                        per_driving_time: 0.0,
                        per_waiting_time: 0.0,
                        per_service_time: 0.0,
                    },
                    dimens,
                    details: vec![VehicleDetail {
                        start: Some(location),
                        end: Some(location),
                        time: Some(time.clone()),
                    }],
                }
            })
            .collect(),
    )
}

pub fn create_dimens_with_id(prefix: &str, id: usize) -> Dimensions {
    let mut dimens = Dimensions::new();
    dimens.set_id([prefix.to_string(), id.to_string()].concat().as_str());
    dimens
}

pub fn create_constraint(activity: Arc<SimpleActivityCost>, transport: Arc<MatrixTransportCost>) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport, 1)));
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));

    constraint
}

pub fn read_init_solution<R: Read>(mut reader: BufReader<R>, problem: Arc<Problem>) -> Result<Solution, String> {
    let mut buffer = String::new();

    let mut solution = Solution {
        registry: Registry::new(&problem.fleet),
        routes: vec![],
        unassigned: Default::default(),
        extras: problem.extras.clone(),
    };

    loop {
        match read_line(&mut reader, &mut buffer) {
            Ok(read) if read > 0 => {
                let route: Vec<_> = buffer.split(':').collect();
                assert_eq!(route.len(), 2);
                let id_map =
                    problem.jobs.all().fold(HashMap::<String, (Arc<Job>, Arc<Single>)>::new(), |mut acc, job| {
                        let single = match job.as_ref() {
                            Job::Single(single) => single.clone(),
                            _ => panic!("Unexpected job type!"),
                        };
                        acc.insert(single.dimens.get_id().unwrap().to_string(), (job.clone(), single));
                        acc
                    });

                let actor = solution.registry.next().next().unwrap();
                let mut tour = Tour::new();
                tour.set_start(create_start_activity(&actor));
                create_end_activity(&actor).map(|end| tour.set_end(end));

                route.last().unwrap().split_whitespace().for_each(|id| {
                    let (job, single) = id_map.get(id).unwrap();
                    let place = single.places.first().unwrap();
                    tour.insert_last(Box::new(Activity {
                        place: core::models::solution::Place {
                            location: place.location.unwrap(),
                            duration: place.duration,
                            time: place.times.first().unwrap().clone(),
                        },
                        schedule: Schedule::new(0.0, 0.0),
                        job: Some(job.clone()),
                    }));
                });

                solution.routes.push(Route { actor, tour });
            }
            Ok(_) => break,
            Err(error) => {
                if buffer.is_empty() {
                    break;
                } else {
                    return Err(error);
                }
            }
        }
    }

    Ok(solution)
}

pub fn read_line<R: Read>(reader: &mut BufReader<R>, mut buffer: &mut String) -> Result<usize, String> {
    buffer.clear();
    reader.read_line(&mut buffer).map_err(|err| err.to_string())
}
