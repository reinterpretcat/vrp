use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::solver::mutation::select_seed_jobs;
use crate::solver::RefinementContext;

/// A ruin strategy which removes jobs in neighbourhood of randomly selected job (inclusive).
pub struct NeighbourRemoval {
    /// Specifies limitation for job removal.
    limits: RuinLimits,
}

impl NeighbourRemoval {
    /// Creates a new instance of `NeighbourRemoval`.
    pub fn new(limits: RuinLimits) -> Self {
        Self { limits }
    }
}

impl Default for NeighbourRemoval {
    fn default() -> Self {
        Self::new(RuinLimits::default())
    }
}

impl Ruin for NeighbourRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.environment.random.clone();

        let routes = insertion_ctx.solution.routes.clone();
        let locked = insertion_ctx.solution.locked.clone();

        let max_removed_activities = self.limits.get_chunk_size(&insertion_ctx);
        let tracker = self.limits.get_tracker();

        select_seed_jobs(&problem, &routes, &random)
            .filter(|job| !locked.contains(job))
            .take_while(|_| tracker.is_not_limit(max_removed_activities))
            .for_each(|job| {
                let route = insertion_ctx.solution.routes.iter_mut().find(|rc| rc.route.tour.contains(&job));

                if let Some(rc) = route {
                    rc.route_mut().tour.remove(&job);
                    insertion_ctx.solution.required.push(job.clone());

                    tracker.add_actor(rc.route.actor.clone());
                    tracker.add_job(job);
                }
            });

        insertion_ctx
    }
}
