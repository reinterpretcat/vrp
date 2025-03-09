use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use crate::solver::search::*;
use std::cell::RefCell;

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

        let tracker = RefCell::new(JobRemovalTracker::new(&self.limits, insertion_ctx.environment.random.as_ref()));
        let mut tabu_list = TabuList::from(&insertion_ctx);

        (0..self.limits.removed_activities_range.end).take_while(|_| !tracker.borrow().is_limit()).for_each(|_| {
            if let Some((_, route_idx, job)) = select_seed_job_with_tabu_list(&insertion_ctx, &tabu_list) {
                if tracker.borrow_mut().try_remove_job(&mut insertion_ctx.solution, route_idx, &job) {
                    tabu_list.add_job(job);
                    tabu_list.add_actor(insertion_ctx.solution.routes[route_idx].route().actor.clone());
                }
            }
        });

        tabu_list.inject(&mut insertion_ctx);

        insertion_ctx
    }
}
