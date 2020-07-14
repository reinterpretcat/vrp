use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

/// A ruin strategy which removes jobs in neighbourhood of randomly selected job (inclusive).
pub struct NeighbourRemoval {
    /// Specifies limitation for job removal.
    limit: JobRemovalLimit,
}

impl NeighbourRemoval {
    /// Creates a new instance of `NeighbourRemoval`.
    pub fn new(limit: JobRemovalLimit) -> Self {
        Self { limit }
    }
}

impl Default for NeighbourRemoval {
    fn default() -> Self {
        Self::new(JobRemovalLimit::default())
    }
}

impl Ruin for NeighbourRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let affected = get_removal_chunk_size(&insertion_ctx, &self.limit);

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
