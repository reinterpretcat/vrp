#[cfg(test)]
#[path = "../../tests/unit/solomon/reader_test.rs"]
mod reader_test;

use crate::common::*;
use crate::utils::MatrixFactory;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::constraints::*;
use vrp_core::models::common::{TimeSpan, TimeWindow};
use vrp_core::models::problem::*;
use vrp_core::models::Problem;

pub fn read_solomon_format<R: Read>(reader: BufReader<R>) -> Result<Problem, String> {
    SolomonReader { buffer: String::new(), reader, matrix: MatrixFactory::default() }.read_problem()
}

/// A trait read write solomon problem.
pub trait SolomonProblem {
    /// Reads solomon problem.
    fn read_solomon(self) -> Result<Problem, String>;
}

impl<R: Read> SolomonProblem for BufReader<R> {
    fn read_solomon(self) -> Result<Problem, String> {
        read_solomon_format(self)
    }
}

impl SolomonProblem for String {
    fn read_solomon(self) -> Result<Problem, String> {
        read_solomon_format(BufReader::new(self.as_bytes()))
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
            depot.tw,
        ))
    }

    fn read_jobs(&mut self) -> Result<Vec<Job>, String> {
        let mut jobs: Vec<Job> = Default::default();
        loop {
            match self.read_customer() {
                Ok(customer) => {
                    let mut dimens = create_dimens_with_id("", customer.id);
                    dimens.set_demand(Demand::<i32> { pickup: (0, 0), delivery: (customer.demand as i32, 0) });
                    jobs.push(Job::Single(Arc::new(Single {
                        places: vec![Place {
                            location: Some(self.matrix.collect(customer.location)),
                            duration: customer.service as f64,
                            times: vec![TimeSpan::Window(customer.tw.clone())],
                        }],
                        dimens,
                    })));
                }
                Err(error) => {
                    if self.buffer.is_empty() {
                        break;
                    } else {
                        return Err(error);
                    }
                }
            }
        }

        Ok(jobs)
    }

    fn create_transport(&self) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
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
            .ok_or_else(|| "Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (id, x, y, demand, start, end, service) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<i32>().unwrap())
            .try_collect()
            .ok_or_else(|| "Cannot read customer line".to_string())?;
        Ok(JobLine {
            id: id as usize,
            location: (x, y),
            demand: demand as usize,
            tw: TimeWindow::new(start as f64, end as f64),
            service: service as usize,
        })
    }

    fn skip_lines(&mut self, count: usize) -> Result<(), String> {
        for _ in 0..count {
            read_line(&mut self.reader, &mut self.buffer).map_err(|_| "Cannot skip lines")?;
        }

        Ok(())
    }
}
