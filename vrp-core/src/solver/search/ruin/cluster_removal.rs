#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/cluster_removal_test.rs"]
mod cluster_removal_test;

use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::search::{get_route_jobs, JobRemovalTracker, TabuList};
use crate::solver::RefinementContext;
use std::cell::RefCell;
use std::sync::Arc;

/// A ruin strategy which removes job clusters using DBSCAN algorithm.
pub struct ClusterRemoval {
    clusters: Vec<Vec<Job>>,
    limits: RemovalLimits,
}

impl ClusterRemoval {
    /// Creates a new instance of `ClusterRemoval`.
    pub fn new(problem: Arc<Problem>, limits: RemovalLimits) -> GenericResult<Self> {
        let clusters = problem
            .jobs
            .clusters()
            .iter()
            .cloned()
            .map(|cluster| cluster.into_iter().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        Ok(Self { clusters, limits })
    }

    /// Creates a new instance of `ClusterRemoval` with default parameters.
    pub fn new_with_defaults(problem: Arc<Problem>) -> GenericResult<Self> {
        let limits = RemovalLimits::new(problem.as_ref());
        Self::new(problem, limits)
    }
}

impl Ruin for ClusterRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let route_jobs = get_route_jobs(&insertion_ctx.solution);
        let tracker = RefCell::new(JobRemovalTracker::new(&self.limits, insertion_ctx.environment.random.as_ref()));
        let mut tabu_list = TabuList::from(&insertion_ctx);

        let mut indices = (0..self.clusters.len()).collect::<Vec<usize>>();
        indices.shuffle(&mut insertion_ctx.environment.random.get_rng());

        indices.into_iter().take_while(|_| !tracker.borrow().is_limit()).for_each(|idx| {
            let cluster = self.clusters.get(idx).unwrap();
            let mut indices = (0..cluster.len()).collect::<Vec<usize>>();
            indices.shuffle(&mut insertion_ctx.environment.random.get_rng());
            indices
                .iter()
                .map(|idx| cluster.get(*idx).expect("invalid cluster index"))
                .take_while(|_| !tracker.borrow().is_limit())
                .for_each(|job| {
                    if let Some(&route_idx) = route_jobs.get(job) {
                        if tracker.borrow_mut().try_remove_job(&mut insertion_ctx.solution, route_idx, job) {
                            tabu_list.add_job(job.clone());
                            tabu_list.add_actor(insertion_ctx.solution.routes[route_idx].route().actor.clone());
                        }
                    }
                });
        });

        // NOTE tabu list is ignored in selection, but it is updated with affected jobs/actors.
        //      this will influence search in other methods
        tabu_list.inject(&mut insertion_ctx);

        insertion_ctx
    }
}
