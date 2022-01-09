#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use super::*;
use crate::construction::clustering::dbscan::create_job_clusters;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::search::get_route_jobs;
use crate::solver::RefinementContext;
use rand::prelude::*;
use rosomaxa::prelude::*;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using DBSCAN algorithm.
pub struct ClusterRemoval {
    clusters: Vec<Vec<Job>>,
    limits: RuinLimits,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>, min_items: usize, limits: RuinLimits) -> Self {
        let mut clusters = create_job_clusters(problem.as_ref(), environment.random.as_ref(), Some(min_items), None);

        clusters.shuffle(&mut environment.random.get_rng());

        Self { clusters, limits }
    }

    /// Creates a new instance of `ClusterRemoval` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        Self::new(problem, environment, 3, RuinLimits::default())
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let locked = insertion_ctx.solution.locked.clone();

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let max_removed_activities = self.limits.get_chunk_size(&insertion_ctx);
        let tracker = self.limits.get_tracker();

        let mut indices = (0..self.clusters.len()).into_iter().collect::<Vec<usize>>();
        indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

        indices.into_iter().take_while(|_| tracker.is_not_limit(max_removed_activities)).for_each(|idx| {
            let cluster = self.clusters.get(idx).unwrap();
            let mut indices = (0..cluster.len()).into_iter().collect::<Vec<usize>>();
            indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

            let left = max_removed_activities - tracker.get_removed_activities();

            indices
                .iter()
                .map(|idx| cluster.get(*idx).expect("invalid cluster index"))
                .filter(|job| !locked.contains(job))
                .take_while(|_| tracker.is_not_limit(max_removed_activities))
                .take(left)
                .for_each(|job| {
                    if let Some(rc) = route_jobs.get_mut(job) {
                        // NOTE actual insertion context modification via route mut
                        if rc.route.tour.contains(job) {
                            rc.route_mut().tour.remove(job);

                            tracker.add_actor(rc.route.actor.clone());
                            tracker.add_job((*job).clone());
                        }
                    }
                });
        });

        tracker.iterate_removed_jobs(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}
