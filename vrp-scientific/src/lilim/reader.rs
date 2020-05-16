#[cfg(test)]
#[path = "../../tests/unit/lilim/reader_test.rs"]
mod reader_test;

use crate::common::*;
use crate::utils::MatrixFactory;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::constraints::{Demand, DemandDimension};
use vrp_core::models::common::{TimeSpan, TimeWindow};
use vrp_core::models::problem::*;
use vrp_core::models::Problem;
use vrp_core::utils::TryCollect;

/// A trait to read lilim problem.
pub trait LilimProblem {
    fn read_lilim(self) -> Result<Problem, String>;
}

impl<R: Read> LilimProblem for BufReader<R> {
    fn read_lilim(self) -> Result<Problem, String> {
        LilimReader { buffer: String::new(), reader: self, matrix: MatrixFactory::default() }.read_problem()
    }
}

impl LilimProblem for String {
    fn read_lilim(self) -> Result<Problem, String> {
        BufReader::new(self.as_bytes()).read_lilim()
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
    demand: i32,
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

impl<R: Read> TextReader for LilimReader<R> {
    fn read_fleet(&mut self) -> Result<Fleet, String> {
        let vehicle = self.read_vehicle()?;
        let depot = self.read_customer()?;

        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.matrix.collect(depot.location),
            depot.tw,
        ))
    }

    fn read_jobs(&mut self) -> Result<Vec<Job>, String> {
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
                        return Err(error);
                    }
                }
            }
        }

        let mut jobs: Vec<Job> = Default::default();
        relations.iter().zip(0..).for_each(|(relation, index)| {
            let pickup = customers.get(&relation.pickup).unwrap();
            let delivery = customers.get(&relation.delivery).unwrap();

            jobs.push(Job::Multi(Multi::bind(Multi::new(
                vec![self.create_single_job(pickup), self.create_single_job(delivery)],
                create_dimens_with_id("mlt", index),
            ))));
        });

        Ok(jobs)
    }

    fn create_transport(&self) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
        self.matrix.create_transport()
    }
}

impl<R: Read> LilimReader<R> {
    fn create_single_job(&mut self, customer: &JobLine) -> Arc<Single> {
        let mut dimens = create_dimens_with_id("c", customer.id);
        dimens.set_demand(if customer.demand > 0 {
            Demand::<i32> { pickup: (0, customer.demand as i32), delivery: (0, 0) }
        } else {
            Demand::<i32> { pickup: (0, 0), delivery: (0, customer.demand as i32) }
        });

        Arc::new(Single {
            places: vec![Place {
                location: Some(self.matrix.collect(customer.location)),
                duration: customer.service as f64,
                times: vec![TimeSpan::Window(customer.tw.clone())],
            }],
            dimens: Default::default(),
        })
    }

    fn read_vehicle(&mut self) -> Result<VehicleLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (number, capacity, _ignored) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or_else(|| "Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity, _ignored })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (id, x, y, demand, start, end, service, _, relation) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<i32>().unwrap())
            .try_collect()
            .ok_or_else(|| "Cannot read customer line".to_string())?;
        Ok(JobLine {
            id: id as usize,
            location: (x, y),
            demand,
            tw: TimeWindow::new(start as f64, end as f64),
            service: service as usize,
            relation: relation as usize,
        })
    }
}
