use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::search::{select_seed_jobs, JobRemovalTracker};
use crate::solver::RefinementContext;

/// A ruin strategy which removes jobs in neighbourhood of randomly selected job (inclusive).
pub struct NeighbourRemoval {
    /// Specifies limitation for job removal.
    limits: RemovalLimits,
}

impl NeighbourRemoval {
    /// Creates a new instance of `NeighbourRemoval`.
    pub fn new(limits: RemovalLimits) -> Self {
        Self { limits }
    }
}

impl Ruin for NeighbourRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.environment.random.clone();

        let routes = insertion_ctx.solution.routes.clone();
        let tracker = RwLock::new(JobRemovalTracker::new(&self.limits, random.as_ref()));

        select_seed_jobs(&problem, &routes, &random).take_while(|_| !tracker.read().unwrap().is_limit()).for_each(
            |job| {
                let route_ctx = routes.iter().find(|rc| rc.route.tour.contains(&job)).cloned();
                if let Some(mut route_ctx) = route_ctx {
                    tracker.write().unwrap().try_remove_job(&mut insertion_ctx.solution, &mut route_ctx, &job);
                }
            },
        );

        insertion_ctx
    }
}
