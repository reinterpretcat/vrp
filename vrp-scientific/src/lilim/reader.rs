#[cfg(test)]
#[path = "../../tests/unit/lilim/reader_test.rs"]
mod reader_test;

use crate::common::*;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::construction::features::JobDemandDimension;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::models::{Extras, Problem};
use vrp_core::prelude::GenericError;
use vrp_core::utils::Float;

/// A trait to read lilim problem.
pub trait LilimProblem {
    /// Reads lilim problem.
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, GenericError>;
}

impl<R: Read> LilimProblem for BufReader<R> {
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, GenericError> {
        LilimReader { buffer: String::new(), reader: self, coord_index: CoordIndex::default() }.read_problem(is_rounded)
    }
}

impl LilimProblem for String {
    fn read_lilim(self, is_rounded: bool) -> Result<Problem, GenericError> {
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
    coord_index: CoordIndex,
}

impl<R: Read> TextReader for LilimReader<R> {
    fn create_goal_context(
        &self,
        activity: Arc<SimpleActivityCost>,
        transport: Arc<dyn TransportCost>,
    ) -> Result<GoalContext, GenericError> {
        let is_time_constrained = true;
        create_goal_context_prefer_min_tours(activity, transport, is_time_constrained)
    }

    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), GenericError> {
        let fleet = self.read_fleet()?;
        let jobs = self.read_jobs()?;

        Ok((jobs, fleet))
    }

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost>, GenericError> {
        self.coord_index.create_transport(is_rounded)
    }

    fn create_extras(&self) -> Extras {
        get_extras(self.coord_index.clone())
    }
}

impl<R: Read> LilimReader<R> {
    fn read_fleet(&mut self) -> Result<Fleet, GenericError> {
        let vehicle = self.read_vehicle()?;
        let depot = self.read_customer()?;

        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.coord_index.collect(depot.location),
            depot.tw,
        ))
    }

    fn read_jobs(&mut self) -> Result<Vec<Job>, GenericError> {
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
                create_dimens_with_id("mlt", &index.to_string(), |id, dimens| {
                    dimens.set_job_id(id.to_string());
                }),
            )));
        });

        Ok(jobs)
    }

    fn create_single_job(&mut self, customer: &JobLine) -> Arc<Single> {
        let mut dimens = create_dimens_with_id("c", &customer.id.to_string(), |id, dimens| {
            dimens.set_job_id(id.to_string());
        });
        dimens.set_job_demand(if customer.demand > 0 {
            Demand::<SingleDimLoad> {
                pickup: (SingleDimLoad::default(), SingleDimLoad::new(customer.demand)),
                delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
            }
        } else {
            Demand::<SingleDimLoad> {
                pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
                delivery: (SingleDimLoad::default(), SingleDimLoad::new(customer.demand)),
            }
        });

        Arc::new(Single {
            places: vec![Place {
                location: Some(self.coord_index.collect(customer.location)),
                duration: customer.service as Float,
                times: vec![TimeSpan::Window(customer.tw.clone())],
            }],
            dimens: Default::default(),
        })
    }

    fn read_vehicle(&mut self) -> Result<VehicleLine, GenericError> {
        read_line(&mut self.reader, &mut self.buffer)?;
        let (number, capacity, _ignored) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect_tuple()
            .ok_or_else(|| "cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity, _ignored })
    }

    fn read_customer(&mut self) -> Result<JobLine, GenericError> {
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
            tw: TimeWindow::new(start as Float, end as Float),
            service: service as usize,
            relation: relation as usize,
        })
    }
}
