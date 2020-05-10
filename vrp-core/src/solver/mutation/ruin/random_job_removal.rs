use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

/// A ruin strategy which removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies limitation for job removal.
    limit: JobRemovalLimit,
}

impl RandomJobRemoval {
    /// Creates a new instance of [`RandomJobRemoval`].
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
    fn run(&self, _refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let affected = get_chunk_size(&insertion_ctx, &self.limit);

        (0..affected).for_each(|_| {
            let solution = &mut insertion_ctx.solution;

            if let Some((route_index, job)) = select_seed_job(&solution.routes, &insertion_ctx.random) {
                if !solution.locked.contains(&job) {
                    solution.routes.get_mut(route_index).unwrap().route_mut().tour.remove(&job);
                    solution.required.push(job);
                }
            }
        });

        insertion_ctx
    }
}
