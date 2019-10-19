#[cfg(test)]
#[path = "../../../tests/unit/refinement/ruin/random_route_removal_test.rs"]
mod random_route_removal_test;

use crate::construction::states::{InsertionContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::refinement::ruin::{create_insertion_context, RuinStrategy};
use crate::refinement::RefinementContext;
use std::collections::HashSet;
use std::sync::Arc;

/// Removes random route from solution.
pub struct RandomRouteRemoval {
    /// Specifies minimum amount of removed routes.
    rmin: f64,
    /// Specifies maximum amount of removed routes.
    rmax: f64,
    /// Specifies threshold ratio of maximum removed routes.
    threshold: f64,
}

impl RandomRouteRemoval {
    pub fn new(rmin: usize, rmax: usize, threshold: f64) -> Self {
        Self { rmin: rmin as f64, rmax: rmax as f64, threshold }
    }

    fn remove_whole_route(&self, solution: &mut SolutionContext, route_ctx: &mut RouteContext) {
        let route = route_ctx.route.read().unwrap();

        solution.routes.remove(route_ctx);
        solution.registry.free_actor(&route.actor);
        solution.required.extend(route.tour.jobs());
    }

    fn remove_part_route(
        &self,
        refinement_ctx: &RefinementContext,
        solution: &mut SolutionContext,
        route_ctx: &mut RouteContext,
    ) {
        let can_remove_full_route =
            route_ctx.route.read().unwrap().tour.jobs().all(|job| !refinement_ctx.locked.contains(&job));

        if can_remove_full_route {
            self.remove_whole_route(solution, route_ctx);
        } else {
            {
                let mut route = route_ctx.route.write().unwrap();
                let jobs: Vec<Arc<Job>> =
                    route.tour.jobs().filter(|job| !refinement_ctx.locked.contains(job)).collect();

                jobs.iter().for_each(|job| {
                    route.tour.remove(job);
                });
                solution.required.extend(jobs);
            }

            refinement_ctx.problem.constraint.accept_route_state(route_ctx);
        }
    }
}

impl Default for RandomRouteRemoval {
    fn default() -> Self {
        Self::new(1, 3, 0.2)
    }
}

impl RuinStrategy for RandomRouteRemoval {
    fn ruin_solution(&self, refinement_ctx: &RefinementContext) -> Result<InsertionContext, String> {
        let individuum = refinement_ctx.individuum()?;
        let solution = individuum.0.as_ref();
        let mut insertion_cxt = create_insertion_context(&refinement_ctx.problem, individuum, &refinement_ctx.random);

        let max = (solution.routes.len() as f64 * self.threshold).max(self.rmin).round() as usize;
        let affected = refinement_ctx
            .random
            .uniform_int(self.rmin as i32, self.rmax as i32 + 1)
            .min(solution.routes.len().min(max) as i32) as usize;

        (0..affected).for_each(|index| {
            let route_index = refinement_ctx.random.uniform_int(0, insertion_cxt.solution.routes.len() as i32) as usize;
            let mut solution = &mut insertion_cxt.solution;
            let mut route_ctx = &mut solution.routes.iter().skip(route_index).next().unwrap().clone();

            if refinement_ctx.locked.is_empty() {
                self.remove_whole_route(solution, route_ctx);
            } else {
                self.remove_part_route(&refinement_ctx, solution, route_ctx);
            }
        });

        Ok(insertion_cxt)
    }
}
