//! This module provides functionality to validate problem definition for logical correctness.

use crate::json::problem::*;

pub struct ValidationContext<'a> {
    pub problem: &'a Problem,
    pub matrices: Option<&'a Vec<Matrix>>,
}

mod jobs;
use self::jobs::validate_jobs;

impl<'a> ValidationContext<'a> {
    /// Creates an instance of `ValidationContext`.
    pub fn new(problem: &'a Problem, matrices: Option<&'a Vec<Matrix>>) -> Self {
        Self { problem, matrices }
    }

    /// Validates problem on set of rules.
    pub fn validate(&self) -> Result<(), String> {
        validate_jobs(&self)
            .map_err(|errors| format!("Problem has the following validation errors: {}", errors.join(",")))
    }

    /// Get list of jobs from the problem.
    fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.problem.plan.jobs.iter()
    }
}
