#[cfg(test)]
#[path = "../../../tests/unit/refinement/ruin/random_route_removal_test.rs"]
mod random_route_removal_test;

use crate::construction::states::{InsertionContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::refinement::ruin::Ruin;
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

        solution.routes.retain(|rc| rc != route_ctx);
        solution.registry.free_actor(&route.actor);
        solution.required.extend(route.tour.jobs());
    }

    fn remove_part_route(&self, insertion_ctx: &mut InsertionContext, route_ctx: &mut RouteContext) {
        let locked = insertion_ctx.locked.clone();
        let solution = &mut insertion_ctx.solution;

        let can_remove_full_route = route_ctx.route.read().unwrap().tour.jobs().all(|job| !locked.contains(&job));

        if can_remove_full_route {
            self.remove_whole_route(solution, route_ctx);
        } else {
            {
                let mut route = route_ctx.route.write().unwrap();
                let jobs: Vec<Arc<Job>> = route.tour.jobs().filter(|job| !locked.contains(job)).collect();

                jobs.iter().for_each(|job| {
                    route.tour.remove(job);
                });
                solution.required.extend(jobs);
            }
        }
    }
}

impl Default for RandomRouteRemoval {
    fn default() -> Self {
        Self::new(1, 3, 0.2)
    }
}

impl Ruin for RandomRouteRemoval {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;
        let max = (insertion_ctx.solution.routes.len() as f64 * self.threshold).max(self.rmin).round() as usize;
        let affected = insertion_ctx
            .random
            .uniform_int(self.rmin as i32, self.rmax as i32)
            .min(insertion_ctx.solution.routes.len().min(max) as i32) as usize;

        (0..affected).for_each(|_| {
            let mut solution = &mut insertion_ctx.solution;
            let route_index = insertion_ctx.random.uniform_int(0, (solution.routes.len() - 1) as i32) as usize;
            let route_ctx = &mut solution.routes.iter().skip(route_index).next().unwrap().clone();

            if insertion_ctx.locked.is_empty() {
                self.remove_whole_route(&mut solution, route_ctx);
            } else {
                self.remove_part_route(&mut insertion_ctx, route_ctx);
            }
        });

        insertion_ctx
    }
}
