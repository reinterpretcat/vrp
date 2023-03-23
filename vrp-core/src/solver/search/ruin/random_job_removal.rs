use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::search::{select_seed_job, JobRemovalTracker};
use crate::solver::RefinementContext;

/// A ruin strategy which removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies limits for job removal.
    limits: RemovalLimits,
}

impl RandomJobRemoval {
    /// Creates a new instance of `RandomJobRemoval`.
    pub fn new(limits: RemovalLimits) -> Self {
        Self { limits }
    }
}

impl Ruin for RandomJobRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let tracker = RwLock::new(JobRemovalTracker::new(&self.limits, insertion_ctx.environment.random.as_ref()));

        (0..self.limits.removed_activities_range.end).take_while(|_| !tracker.read().unwrap().is_limit()).for_each(
            |_| {
                if let Some((route_index, job)) =
                    select_seed_job(&insertion_ctx.solution.routes, &insertion_ctx.environment.random)
                {
                    let mut route_ctx = insertion_ctx.solution.routes.get_mut(route_index).unwrap().clone();
                    tracker.write().unwrap().try_remove_job(&mut insertion_ctx.solution, &mut route_ctx, &job);
                }
            },
        );

        insertion_ctx
    }
}
