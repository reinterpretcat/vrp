#[cfg(test)]
#[path = "../../tests/unit/lilim/reader_test.rs"]
mod reader_test;

use crate::common::*;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::{Extras, Problem};

/// A trait to read lilim problem.
pub trait LilimProblem {
    /// Reads lilim problem.
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, String>;
}

impl<R: Read> LilimProblem for BufReader<R> {
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, String> {
        LilimReader { buffer: String::new(), reader: self, matrix: CoordIndex::default() }.read_problem(is_rounded)
    }
}

impl LilimProblem for String {
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, String> {
        BufReader::new(self.as_bytes()).read_lilim(is_rounded)
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
    matrix: CoordIndex,
}

impl<R: Read> TextReader for LilimReader<R> {
    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), String> {
        let fleet = self.read_fleet()?;
        let jobs = self.read_jobs()?;

        Ok((jobs, fleet))
    }

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
        self.matrix.create_transport(is_rounded)
    }

    fn create_extras(&self) -> Extras {
        Extras::default()
    }
}

impl<R: Read> LilimReader<R> {
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

            jobs.push(Job::Multi(Multi::new_shared(
                vec![self.create_single_job(pickup), self.create_single_job(delivery)],
                create_dimens_with_id("mlt", &index.to_string()),
            )));
        });

        Ok(jobs)
    }

    fn create_single_job(&mut self, customer: &JobLine) -> Arc<Single> {
        let mut dimens = create_dimens_with_id("c", &customer.id.to_string());
        dimens.set_demand(if customer.demand > 0 {
            Demand::<SingleDimLoad> {
                pickup: (SingleDimLoad::default(), SingleDimLoad::new(customer.demand as i32)),
                delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
            }
        } else {
            Demand::<SingleDimLoad> {
                pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
                delivery: (SingleDimLoad::default(), SingleDimLoad::new(customer.demand as i32)),
            }
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
            .try_collect_tuple()
            .ok_or_else(|| "cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity, _ignored })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (id, x, y, demand, start, end, service, _, relation) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<i32>().unwrap())
            .try_collect_tuple()
            .ok_or_else(|| "cannot read customer line".to_string())?;
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
