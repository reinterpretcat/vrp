//! This module provides functionality to validate problem definition for logical correctness.

use crate::format::problem::*;
use crate::format::{CoordIndex, FormatError};

/// A validation context which keeps essential information.
pub struct ValidationContext<'a> {
    /// An original problem.
    pub problem: &'a Problem,
    /// Routing matrices.
    pub matrices: Option<&'a Vec<Matrix>>,

    coord_index: CoordIndex,
    job_index: HashMap<String, Job>,
}

mod common;
use self::common::*;

mod jobs;
use self::jobs::validate_jobs;

mod objectives;
use self::objectives::validate_objectives;

mod vehicles;
use self::vehicles::validate_vehicles;

mod relations;
use self::relations::validate_relations;

mod routing;
use self::routing::validate_routing;
use hashbrown::HashMap;

impl<'a> ValidationContext<'a> {
    /// Creates an instance of `ValidationContext`.
    pub fn new(problem: &'a Problem, matrices: Option<&'a Vec<Matrix>>) -> Self {
        Self {
            problem,
            matrices,
            coord_index: CoordIndex::new(problem),
            job_index: problem.plan.jobs.iter().map(|job| (job.id.clone(), job.clone())).collect(),
        }
    }

    /// Validates problem on set of rules.
    pub fn validate(&self) -> Result<(), Vec<FormatError>> {
        let errors = validate_jobs(&self)
            .err()
            .into_iter()
            .chain(validate_vehicles(&self).err().into_iter())
            .chain(validate_objectives(&self).err().into_iter())
            .chain(validate_routing(&self).err().into_iter())
            .chain(validate_relations(&self).err().into_iter())
            .flatten()
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Gets list of jobs from the problem.
    fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.problem.plan.jobs.iter()
    }

    /// Gets list of vehicles from the problem.
    fn vehicles(&self) -> impl Iterator<Item = &VehicleType> {
        self.problem.fleet.vehicles.iter()
    }

    /// Gets a flat list of job tasks from the job.
    fn tasks(&self, job: &'a Job) -> Vec<&'a JobTask> {
        job.pickups
            .as_ref()
            .iter()
            .flat_map(|tasks| tasks.iter())
            .chain(job.deliveries.as_ref().iter().flat_map(|tasks| tasks.iter()))
            .chain(job.replacements.as_ref().iter().flat_map(|tasks| tasks.iter()))
            .chain(job.services.as_ref().iter().flat_map(|tasks| tasks.iter()))
            .collect()
    }
}

fn combine_error_results(results: &[Result<(), FormatError>]) -> Result<(), Vec<FormatError>> {
    let errors = results.iter().cloned().flat_map(|result| result.err().into_iter()).collect::<Vec<FormatError>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn is_reserved_job_id(job_id: &str) -> bool {
    job_id == "departure" || job_id == "arrival" || job_id == "break" || job_id == "reload" || job_id == "depot"
}
