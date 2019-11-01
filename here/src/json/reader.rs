use core::models::Problem;
use std::fs::File;
use std::io::BufReader;

#[path = "./deserializer.rs"]
mod deserializer;
use self::deserializer::*;
type ApiProblem = self::deserializer::Problem;

#[path = "./utils.rs"]
mod utils;
use self::utils::*;
use crate::json::coord_index::CoordIndex;

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
