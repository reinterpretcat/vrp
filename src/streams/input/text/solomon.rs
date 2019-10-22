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

struct VehicleLine {
    number: usize,
    capacity: usize,
}

struct JobLine {
    id: usize,
    location: (i32, i32),
    demand: usize,
    tw: TimeWindow,
    service: usize,
}

struct SolomonReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    matrix: MatrixFactory,
}

impl<R: Read> TextReader for SolomonReader<R> {
    fn read_fleet(&mut self) -> Result<Fleet, String> {
        self.skip_lines(4)?;
        let vehicle = self.read_vehicle()?;
        self.skip_lines(4)?;
        let depot = self.read_customer()?;
        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.matrix.collect(depot.location),
            depot.tw.clone(),
        ))
    }

    fn read_jobs(&mut self, fleet: &Fleet) -> Result<Vec<Arc<Job>>, String> {
        let mut jobs: Vec<Arc<Job>> = Default::default();
        loop {
            match self.read_customer() {
                Ok(customer) => {
                    let mut dimens = create_dimens_with_id("", customer.id);
                    dimens.set_demand(Demand::<i32> { pickup: (0, 0), delivery: (customer.demand as i32, 0) });
                    jobs.push(Arc::new(Job::Single(Arc::new(Single {
                        places: vec![Place {
                            location: Some(self.matrix.collect(customer.location)),
                            duration: customer.service as f64,
                            times: vec![customer.tw.clone()],
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

    fn create_transport(&self) -> MatrixTransportCost {
        self.matrix.create_transport()
    }
}

impl<R: Read> SolomonReader<R> {
    fn read_vehicle(&mut self) -> Result<VehicleLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (number, capacity) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (id, x, y, demand, start, end, service) = self
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
        })
    }

    fn skip_lines(&mut self, count: usize) -> Result<(), String> {
        for i in 0..count {
            read_line(&mut self.reader, &mut self.buffer).map_err(|_| "Cannot skip lines")?;
        }

        Ok(())
    }
}
