//! This module provides functionality to validate problem definition for logical correctness.

use crate::json::problem::*;

struct ValidationContext<'a> {
    pub problem: &'a Problem,
    pub matrices: &'a Vec<Matrix>,
}

impl<'a> ValidationContext<'a> {
    /// Get list of jobs from the problem.
    fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.problem.plan.jobs.iter()
    }
}

mod jobs;
