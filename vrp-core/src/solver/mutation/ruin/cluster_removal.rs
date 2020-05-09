#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use crate::construction::heuristics::InsertionContext;
use crate::models::Problem;
use crate::solver::mutation::Ruin;
use crate::solver::RefinementContext;
use std::ops::Range;
use std::sync::Arc;

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

    pub fn new_from_problem(problem: Arc<Problem>) -> Self {
        unimplemented!()
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &mut RefinementContext, _: InsertionContext) -> InsertionContext {
        // TODO eps_range: get few random random jobs and check their average neighborhood?
        //      select_seed_jobs
        // TODO min points: use activities (jobs?) amount?

        // TODO estimate epsilon
        //      for each point p on the curve, we find the one with the maximum distance d to a
        //      line drawn from the first to the last point of the curve

        unimplemented!()
    }
}
