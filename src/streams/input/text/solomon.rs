#[cfg(test)]
#[path = "../../../../tests/unit/streams/input/text/solomon_test.rs"]
mod solomon_test;

use crate::construction::constraints::*;
use crate::models::common::{Dimensions, IdDimension, Location, TimeWindow};
use crate::models::problem::*;
use crate::models::Problem;
use crate::streams::input::text::*;
use crate::utils::{MatrixFactory, TryCollect};
use std::borrow::Borrow;
use std::fs::{read, File};
use std::io::prelude::*;
use std::io::{BufReader, Error};
use std::sync::Arc;

pub fn read_solomon_format<R: Read>(mut reader: BufReader<R>) -> Result<Problem, String> {
    SolomonReader { buffer: String::new(), reader, matrix: MatrixFactory::new() }.read_problem()
}

pub trait SolomonProblem {
    fn parse_solomon(&self) -> Result<Problem, String>;
}

impl SolomonProblem for File {
    fn parse_solomon(&self) -> Result<Problem, String> {
        read_solomon_format(BufReader::new(self))
    }
}

impl SolomonProblem for String {
    fn parse_solomon(&self) -> Result<Problem, String> {
        read_solomon_format(BufReader::new(StringReader::new(self.as_str())))
    }
}

struct SolomonReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    matrix: MatrixFactory,
}

struct VehicleLine {
    number: usize,
    capacity: usize,
}

struct JobLine {
    id: usize,
    location: (usize, usize),
    demand: usize,
    start: usize,
    end: usize,
    service: usize,
}

impl<R: Read> SolomonReader<R> {
    pub fn read_problem(&mut self) -> Result<Problem, String> {
        let fleet = self.read_fleet()?;
        let jobs = self.read_jobs(&fleet)?;
        let transport = Arc::new(self.matrix.create_transport());
        let activity = Arc::new(SimpleActivityCost::new());
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

    fn read_fleet(&mut self) -> Result<Fleet, String> {
        self.skip_lines(4)?;
        let vehicle = self.read_vehicle()?;
        self.skip_lines(4)?;
        let depot = self.read_customer()?;
        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.matrix.collect(depot.location),
            TimeWindow { start: depot.start as f64, end: depot.end as f64 },
        ))
    }

    fn read_jobs(&mut self, fleet: &Fleet) -> Result<Vec<Arc<Job>>, String> {
        let mut jobs: Vec<Arc<Job>> = Default::default();
        loop {
            match self.read_customer() {
                Ok(customer) => {
                    let mut dimens = create_dimens_with_id("c", customer.id);
                    dimens.set_demand(Demand::<i32> { pickup: (0, 0), delivery: (customer.demand as i32, 0) });
                    jobs.push(Arc::new(Job::Single(Arc::new(Single {
                        places: vec![Place {
                            location: Some(self.matrix.collect(customer.location)),
                            duration: customer.service as f64,
                            times: vec![TimeWindow { start: customer.start as f64, end: customer.end as f64 }],
                        }],
                        dimens,
                    }))));
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

        Ok(jobs)
    }

    fn read_vehicle(&mut self) -> Result<VehicleLine, String> {
        self.read_line()?;
        let (number, capacity) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        self.read_line()?;
        let (id, x, y, demand, start, end, service) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot read customer line".to_string())?;
        Ok(JobLine { id, location: (x, y), demand, start, end, service })
    }

    fn skip_lines(&mut self, count: usize) -> Result<(), String> {
        for i in 0..count {
            self.read_line().map_err(|_| "Cannot skip lines")?;
        }

        Ok(())
    }

    fn read_line(&mut self) -> Result<usize, String> {
        self.buffer.clear();
        self.reader.read_line(&mut self.buffer).map_err(|err| err.to_string())
    }
}
