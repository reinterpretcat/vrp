//! Specifies logic to read problem and routing matrix from json input.

use super::*;
use crate::parse_time;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::TimeWindow;
use vrp_core::utils::*;

pub(crate) type ApiProblem = Problem;

mod model;
pub use self::model::*;

#[cfg(test)]
#[path = "../../../tests/unit/format/problem/reader_test.rs"]
mod reader_test;

mod clustering_reader;

mod fleet_reader;
pub use self::fleet_reader::create_approx_matrices;

mod goal_reader;
mod job_reader;

mod problem_reader;
use self::problem_reader::{map_to_problem_with_approx, map_to_problem_with_matrices};

/// Reads specific problem definition from various sources.
pub trait PragmaticProblem {
    /// Reads problem defined in pragmatic format.
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>>;
}

impl<R: Read> PragmaticProblem for (BufReader<R>, Vec<BufReader<R>>) {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        let problem = deserialize_problem(self.0)?;

        let mut matrices = vec![];
        for matrix in self.1 {
            matrices.push(deserialize_matrix(matrix)?);
        }

        map_to_problem_with_matrices(problem, matrices)
    }
}

impl<R: Read> PragmaticProblem for BufReader<R> {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        let problem = deserialize_problem(self)?;

        map_to_problem_with_approx(problem)
    }
}

impl PragmaticProblem for (String, Vec<String>) {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        let problem = deserialize_problem(BufReader::new(self.0.as_bytes()))?;

        let mut matrices = vec![];
        for matrix in self.1 {
            matrices.push(deserialize_matrix(BufReader::new(matrix.as_bytes()))?);
        }

        map_to_problem_with_matrices(problem, matrices)
    }
}

impl PragmaticProblem for String {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        let problem = deserialize_problem(BufReader::new(self.as_bytes()))?;

        map_to_problem_with_approx(problem)
    }
}

impl PragmaticProblem for (ApiProblem, Vec<Matrix>) {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        map_to_problem_with_matrices(self.0, self.1)
    }
}

impl PragmaticProblem for ApiProblem {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        map_to_problem_with_approx(self)
    }
}

impl PragmaticProblem for (ApiProblem, Option<Vec<Matrix>>) {
    fn read_pragmatic(self) -> Result<CoreProblem, Vec<FormatError>> {
        if let Some(matrices) = self.1 {
            (self.0, matrices).read_pragmatic()
        } else {
            self.0.read_pragmatic()
        }
    }
}

pub(crate) fn get_job_tasks(job: &Job) -> impl Iterator<Item = &JobTask> {
    job.pickups.iter().chain(job.deliveries.iter()).chain(job.services.iter()).chain(job.replacements.iter()).flatten()
}

pub(crate) struct ProblemProperties {
    has_multi_dimen_capacity: bool,
    has_breaks: bool,
    has_skills: bool,
    has_unreachable_locations: bool,
    has_dispatch: bool,
    has_reloads: bool,
    has_order: bool,
    has_group: bool,
    has_compatibility: bool,
    has_tour_size_limits: bool,
    has_tour_travel_limits: bool,
}

fn parse_time_window(tw: &[String]) -> TimeWindow {
    assert_eq!(tw.len(), 2);
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}
