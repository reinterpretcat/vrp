//! This module provides functionality to validate problem definition for logical correctness.

use crate::json::problem::*;

pub struct ValidationContext<'a> {
    pub problem: &'a Problem,
    pub matrices: Option<&'a Vec<Matrix>>,
}

/// A validation error.
#[derive(Clone)]
pub struct ValidationError {
    /// A documentation error code.
    pub code: String,
    /// A possible error cause.
    pub cause: String,
    /// An action to take in order to recover from error.
    pub action: String,
}

mod common;
use self::common::*;

mod jobs;
use self::jobs::validate_jobs;

mod objectives;
use self::objectives::validate_objectives;

mod vehicles;
use self::vehicles::validate_vehicles;

impl<'a> ValidationContext<'a> {
    /// Creates an instance of `ValidationContext`.
    pub fn new(problem: &'a Problem, matrices: Option<&'a Vec<Matrix>>) -> Self {
        Self { problem, matrices }
    }

    /// Validates problem on set of rules.
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let errors = validate_jobs(&self)
            .err()
            .into_iter()
            .chain(validate_vehicles(&self).err().into_iter())
            .chain(validate_objectives(&self).err().into_iter())
            .flatten()
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get list of jobs from the problem.
    fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.problem.plan.jobs.iter()
    }

    /// Get list of vehicles from the problem.
    fn vehicles(&self) -> impl Iterator<Item = &VehicleType> {
        self.problem.fleet.vehicles.iter()
    }
}

impl ValidationError {
    /// Creates a new instance of `ValidationError` action.
    pub fn new(code: String, cause: String, action: String) -> Self {
        Self { code, cause, action }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}, cause: '{}', action: '{}'.", self.code, self.cause, self.action)
    }
}
