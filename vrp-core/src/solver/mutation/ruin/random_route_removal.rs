#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/random_route_removal_test.rs"]
mod random_route_removal_test;

use super::*;
use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::solver::RefinementContext;

/// A ruin strategy which removes random route from solution.
pub struct RandomRouteRemoval {
    /// Specifies minimum amount of removed routes.
    min: f64,
    /// Specifies maximum amount of removed routes.
    max: f64,
    /// Specifies threshold ratio of maximum removed routes.
    threshold: f64,
}

impl RandomRouteRemoval {
    /// Creates a new instance of `RandomRouteRemoval`.
    pub fn new(rmin: usize, rmax: usize, threshold: f64) -> Self {
        Self { min: rmin as f64, max: rmax as f64, threshold }
    }

    fn remove_whole_route(&self, solution: &mut SolutionContext, route_ctx: &mut RouteContext) {
        solution.routes.retain(|rc| rc != route_ctx);
        solution.registry.free_actor(&route_ctx.route.actor);
        solution.required.extend(route_ctx.route.tour.jobs());
    }

    fn remove_part_route(&self, insertion_ctx: &mut InsertionContext, route_ctx: &mut RouteContext) {
        let solution = &mut insertion_ctx.solution;
        let locked = solution.locked.clone();

        let can_remove_full_route = route_ctx.route.tour.jobs().all(|job| !locked.contains(&job));

        if can_remove_full_route {
            self.remove_whole_route(solution, route_ctx);
        } else {
            {
                let jobs: Vec<Job> = route_ctx.route.tour.jobs().filter(|job| !locked.contains(job)).collect();

                jobs.iter().for_each(|job| {
                    route_ctx.route_mut().tour.remove(job);
                });
                solution.required.extend(jobs);
            }
        }
    }
}

impl Default for RandomRouteRemoval {
    fn default() -> Self {
        Self::new(1, 4, 0.1)
    }
}

impl Ruin for RandomRouteRemoval {
    fn run(&self, _refinement_ctx: &mut RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let max = (insertion_ctx.solution.routes.len() as f64 * self.threshold).max(self.min).round() as usize;
        let affected = insertion_ctx
            .random
            .uniform_int(self.min as i32, self.max as i32)
            .min(insertion_ctx.solution.routes.len().min(max) as i32) as usize;

        (0..affected).for_each(|_| {
            let mut solution = &mut insertion_ctx.solution;
            let route_index = insertion_ctx.random.uniform_int(0, (solution.routes.len() - 1) as i32) as usize;
            let route_ctx = &mut solution.routes.get(route_index).unwrap().clone();

            if solution.locked.is_empty() {
                self.remove_whole_route(&mut solution, route_ctx);
            } else {
                self.remove_part_route(&mut insertion_ctx, route_ctx);
            }
        });

        insertion_ctx
    }
}
