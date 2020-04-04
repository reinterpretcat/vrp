use super::{get_chunk_size, select_seed_job, Ruin};
use crate::construction::heuristics::InsertionContext;
use crate::refinement::RefinementContext;

/// A ruin strategy which removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies minimum and maximum amount of removed jobs.
    range: (usize, usize),
    /// Specifies threshold ratio of maximum removed jobs.
    threshold: f64,
}

impl RandomJobRemoval {
    /// Creates a new instance of [`RandomJobRemoval`].
    pub fn new(min: usize, max: usize, threshold: f64) -> Self {
        Self { range: (min, max), threshold }
    }
}

impl Default for RandomJobRemoval {
    fn default() -> Self {
        Self::new(1, 10, 0.2)
    }
}

impl Ruin for RandomJobRemoval {
    fn run(&self, _refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let affected = get_chunk_size(&insertion_ctx, &self.range, self.threshold);

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
