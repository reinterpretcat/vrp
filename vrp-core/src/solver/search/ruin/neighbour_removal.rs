use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::search::*;
use crate::solver::RefinementContext;
use std::cell::RefCell;

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
        let tracker = RefCell::new(JobRemovalTracker::new(&self.limits, &random));
        let mut tabu_list = TabuList::from(&insertion_ctx);

        let init_seed =
            select_seed_job_with_tabu_list(&insertion_ctx, &tabu_list).map(|(profile, _, job)| (profile, job));

        select_neighbors(&problem, init_seed).take_while(|_| !tracker.borrow().is_limit()).for_each(|job| {
            let route_idx =
                insertion_ctx.solution.routes.iter().position(|route_ctx| route_ctx.route().tour.contains(&job));
            if let Some(route_idx) = route_idx {
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
