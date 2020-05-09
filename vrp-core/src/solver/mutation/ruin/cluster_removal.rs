#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::Ruin;
use crate::solver::RefinementContext;
use std::ops::Range;

/// A ruin strategy which removes job clusters using DBSCAN algorithm.
pub struct ClusterRemoval {
    /// A range parameter for the distance which defines the neighborhood of a job.
    _eps_range: Range<f64>,
    /// A range parameter for minimum amount of the jobs to form the cluster.
    _min_point_range: Range<usize>,
}

impl ClusterRemoval {
    pub fn new(eps_range: Range<f64>, min_point_range: Range<usize>) -> Self {
        Self { _eps_range: eps_range, _min_point_range: min_point_range }
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &mut RefinementContext, _: InsertionContext) -> InsertionContext {
        // TODO eps_range: get few random random jobs and check their average neighborhood?
        //      select_seed_jobs
        // TODO min points: use activities (jobs?) amount?

        unimplemented!()
    }
}
