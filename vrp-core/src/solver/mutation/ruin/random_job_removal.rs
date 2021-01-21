use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::{get_selection_chunk_size, select_seed_job};
use crate::solver::RefinementContext;

/// A ruin strategy which removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies limitation for job removal.
    limit: JobRemovalLimit,
}

impl RandomJobRemoval {
    /// Creates a new instance of `RandomJobRemoval`.
    pub fn new(limit: JobRemovalLimit) -> Self {
        Self { limit }
    }
}

impl Default for RandomJobRemoval {
    fn default() -> Self {
        Self::new(JobRemovalLimit::default())
    }
}

impl Ruin for RandomJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let affected = get_selection_chunk_size(&insertion_ctx, self.limit.min, self.limit.max, self.limit.threshold);

        (0..affected).for_each(|_| {
            let solution = &mut insertion_ctx.solution;

            if let Some((route_index, job)) = select_seed_job(&solution.routes, &insertion_ctx.environment.random) {
                if !solution.locked.contains(&job) {
                    solution.routes.get_mut(route_index).unwrap().route_mut().tour.remove(&job);
                    solution.required.push(job);
                }
            }
        });

        insertion_ctx
    }
}
