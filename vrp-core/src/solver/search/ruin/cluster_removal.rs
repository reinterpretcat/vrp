#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use super::*;
use crate::construction::clustering::dbscan::create_job_clusters;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::search::{get_route_jobs, JobRemovalTracker};
use crate::solver::RefinementContext;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using DBSCAN algorithm.
pub struct ClusterRemoval {
    clusters: Vec<Vec<Job>>,
    limits: RemovalLimits,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>, min_items: usize, limits: RemovalLimits) -> Self {
        let mut clusters = create_job_clusters(problem.as_ref(), environment.random.as_ref(), Some(min_items), None);

        clusters.shuffle(&mut environment.random.get_rng());

        Self { clusters, limits }
    }

    /// Creates a new instance of `ClusterRemoval` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        let limits = RemovalLimits::new(problem.as_ref());
        Self::new(problem, environment, 3, limits)
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let route_jobs = get_route_jobs(&insertion_ctx.solution);
        let tracker = RwLock::new(JobRemovalTracker::new(&self.limits, insertion_ctx.environment.random.as_ref()));

        let mut indices = (0..self.clusters.len()).collect::<Vec<usize>>();
        indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

        indices.into_iter().take_while(|_| !tracker.read().unwrap().is_limit()).for_each(|idx| {
            let cluster = self.clusters.get(idx).unwrap();
            let mut indices = (0..cluster.len()).collect::<Vec<usize>>();
            indices.shuffle(&mut insertion_ctx.environment.random.get_rng());
            indices
                .iter()
                .map(|idx| cluster.get(*idx).expect("invalid cluster index"))
                .take_while(|_| !tracker.read().unwrap().is_limit())
                .for_each(|job| {
                    if let Some(route_idx) = route_jobs.get(job) {
                        tracker.write().unwrap().try_remove_job(&mut insertion_ctx.solution, *route_idx, job);
                    }
                });
        });

        insertion_ctx
    }
}
