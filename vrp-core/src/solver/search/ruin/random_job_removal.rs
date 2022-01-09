use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::search::select_seed_job;
use crate::solver::RefinementContext;

/// A ruin strategy which removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies limitation for job removal.
    limits: RuinLimits,
}

impl RandomJobRemoval {
    /// Creates a new instance of `RandomJobRemoval`.
    pub fn new(limits: RuinLimits) -> Self {
        Self { limits }
    }
}

impl Default for RandomJobRemoval {
    fn default() -> Self {
        Self::new(RuinLimits::default())
    }
}

impl Ruin for RandomJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let affected = self.limits.get_chunk_size(&insertion_ctx);

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
