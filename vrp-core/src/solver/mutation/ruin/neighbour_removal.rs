use super::{get_chunk_size, select_seed_jobs, Ruin};
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

/// A ruin strategy which removes jobs in neighbourhood of randomly selected job (inclusive).
pub struct NeighbourRemoval {
    /// Specifies minimum and maximum amount of removed jobs.
    range: (usize, usize),
    /// Specifies threshold ratio of maximum removed jobs.
    threshold: f64,
}

impl NeighbourRemoval {
    /// Creates a new instance of [`NeighbourRemoval`].
    pub fn new(min: usize, max: usize, threshold: f64) -> Self {
        Self { range: (min, max), threshold }
    }
}

impl Default for NeighbourRemoval {
    fn default() -> Self {
        Self::new(15, 30, 0.5)
    }
}

impl Ruin for NeighbourRemoval {
    fn run(&self, _refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        let affected = get_chunk_size(&insertion_ctx, &self.range, self.threshold);

        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.random.clone();

        let routes = insertion_ctx.solution.routes.clone();
        let locked = insertion_ctx.solution.locked.clone();

        select_seed_jobs(&problem, &routes, &random).filter(|job| !locked.contains(job)).take(affected).for_each(
            |job| {
                let route = insertion_ctx.solution.routes.iter_mut().find(|rc| rc.route.tour.contains(&job));

                if let Some(route) = route {
                    route.route_mut().tour.remove(&job);
                    insertion_ctx.solution.required.push(job);
                }
            },
        );

        insertion_ctx
    }
}
