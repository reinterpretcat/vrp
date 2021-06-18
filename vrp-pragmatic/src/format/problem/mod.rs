//! Specifies logic to read problem and routing matrix from json input.
//!

mod model;
pub use self::model::*;

mod reader;
pub use self::reader::PragmaticProblem;

pub(crate) fn get_job_tasks(jobs: &'_ [Job]) -> impl Iterator<Item = &'_ JobTask> + '_ {
    jobs.iter()
        .flat_map(|job| {
            job.pickups.iter().chain(job.deliveries.iter()).chain(job.services.iter()).chain(job.replacements.iter())
        })
        .flatten()
}
