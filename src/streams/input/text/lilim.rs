#[cfg(test)]
#[path = "../../../../tests/unit/streams/input/text/lilim_test.rs"]
mod lilim_test;

use crate::construction::constraints::Demand;
use crate::models::common::TimeWindow;
use crate::models::problem::*;
use crate::models::Problem;
use crate::streams::input::text::*;
use crate::utils::{MatrixFactory, TryCollect};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Read};
use std::sync::Arc;

pub fn read_lilim_format<R: Read>(mut reader: BufReader<R>) -> Result<Problem, String> {
    LilimReader { buffer: String::new(), reader, matrix: MatrixFactory::new() }.read_problem()
}

pub trait LilimProblem {
    fn parse_lilim(&self) -> Result<Problem, String>;
}

impl LilimProblem for File {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(self))
    }
}

impl LilimProblem for String {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(StringReader::new(self.as_str())))
    }
}

struct VehicleLine {
    number: usize,
    capacity: usize,
    _ignored: usize,
}

struct JobLine {
    id: usize,
    location: (i32, i32),
    demand: usize,
    tw: TimeWindow,
    service: usize,
    relation: usize,
}

struct Relation {
    pickup: usize,
    delivery: usize,
}

struct LilimReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    matrix: MatrixFactory,
}

impl<R: Read> LilimReader<R> {
    pub fn read_problem(&mut self) -> Result<Problem, String> {
        let fleet = self.read_fleet()?;
        let jobs = self.read_jobs(&fleet)?;
        let transport = Arc::new(self.matrix.create_transport());
        let activity = Arc::new(SimpleActivityCost::new());
        let jobs = Jobs::new(&fleet, jobs, transport.as_ref());

        unimplemented!()
    }

    fn read_fleet(&mut self) -> Result<Fleet, String> {
        let vehicle = self.read_vehicle()?;
        let depot = self.read_customer()?;

        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.matrix.collect(depot.location),
            depot.tw.clone(),
        ))
    }

    fn read_jobs(&mut self, fleet: &Fleet) -> Result<Vec<Arc<Job>>, String> {
        let mut customers: HashMap<usize, JobLine> = Default::default();
        let mut relations: Vec<Relation> = Default::default();
        loop {
            match self.read_customer() {
                Ok(customer) => {
                    if customer.demand > 0 {
                        relations.push(Relation { pickup: customer.id, delivery: customer.relation });
                    }
                    customers.insert(customer.id, customer);
                }
                Err(error) => {
                    if self.buffer.is_empty() {
                        break;
                    } else {
                        Err(error)?;
                    }
                }
            }
        }

        let mut jobs: Vec<Arc<Job>> = Default::default();
        relations.iter().zip(0..).for_each(|(relation, index)| {
            let pickup = customers.get(&relation.pickup).unwrap();
            let delivery = customers.get(&relation.delivery).unwrap();
            let seq_id = "mlt".to_string() + index.to_string().as_str();

            let singles: Vec<Arc<Single>> = vec![Arc::new(Single {
                places: vec![Place {
                    location: Some(self.matrix.collect(pickup.location)),
                    duration: pickup.service as f64,
                    times: vec![pickup.tw.clone()],
                }],
                dimens: Default::default(),
            })];

            jobs.push(Arc::new(Job::Multi(Arc::new(Multi::new(singles, Default::default())))));
        });

        Ok(jobs)
    }

    fn read_vehicle(&mut self) -> Result<VehicleLine, String> {
        self.read_line()?;
        let (number, capacity, _ignored) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity, _ignored })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        self.read_line()?;
        let (id, x, y, demand, start, end, service, _, relation) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<i32>().unwrap())
            .try_collect()
            .ok_or("Cannot read customer line".to_string())?;
        Ok(JobLine {
            id: id as usize,
            location: (x, y),
            demand: demand as usize,
            tw: TimeWindow::new(start as f64, end as f64),
            service: service as usize,
            relation: relation as usize,
        })
    }

    fn read_line(&mut self) -> Result<usize, String> {
        self.buffer.clear();
        self.reader.read_line(&mut self.buffer).map_err(|err| err.to_string())
    }
}
