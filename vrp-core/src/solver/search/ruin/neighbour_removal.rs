use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::search::{select_neighbors, select_seed_job, JobRemovalTracker};
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
        let tracker = RwLock::new(JobRemovalTracker::new(&self.limits, random.as_ref()));

        let init_seed = select_seed_job(insertion_ctx.solution.routes.as_slice(), random.as_ref())
            .map(|(profile, _, job)| (profile, job));

        select_neighbors(&problem, init_seed).take_while(|_| !tracker.read().unwrap().is_limit()).for_each(|job| {
            let route_idx =
                insertion_ctx.solution.routes.iter().position(|route_ctx| route_ctx.route().tour.contains(&job));
            if let Some(route_idx) = route_idx {
                tracker.write().unwrap().try_remove_job(&mut insertion_ctx.solution, route_idx, &job);
            }
        });

        insertion_ctx
    }
}
