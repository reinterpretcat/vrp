use crate::json::solution::serialize_solution;
use crate::json::solution::serializer::Timing;
use core::models::common::{Cost, Distance, Duration, Location, Timestamp};
use core::models::{Problem, Solution};
use std::io::{BufWriter, Write};

type ApiSolution = crate::json::solution::serializer::Solution;

pub trait HereSolution<W: Write> {
    fn write_here(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> HereSolution<W> for Solution {
    fn write_here(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String> {
        let solution = create_solution(problem, &self);
        serialize_solution(writer, &solution).map_err(|err| err.to_string())?;
        Ok(())
    }
}

struct Leg {
    pub location: Location,
    pub departure: Timestamp,
    pub distance: Distance,
    pub duration: Duration,
    pub timing: Timing,
    pub cost: Cost,
    pub load: i32,
}

fn create_solution(_problem: &Problem, _solution: &Solution) -> ApiSolution {
    unimplemented!()
}
