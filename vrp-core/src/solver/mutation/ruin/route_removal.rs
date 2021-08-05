#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/route_removal_test.rs"]
mod route_removal_test;

use super::*;
use crate::construction::heuristics::{group_routes_by_proximity, InsertionContext, RouteContext, SolutionContext};
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
}

impl Default for RandomRouteRemoval {
    fn default() -> Self {
        Self::new(1, 4, 0.1)
    }
}

impl Ruin for RandomRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let random = insertion_ctx.environment.random.clone();
        let max = (insertion_ctx.solution.routes.len() as f64 * self.threshold).max(self.min).round() as usize;
        let affected = random
            .uniform_int(self.min as i32, self.max as i32)
            .min(insertion_ctx.solution.routes.len().min(max) as i32) as usize;

        (0..affected).for_each(|_| {
            let route_index = random.uniform_int(0, (insertion_ctx.solution.routes.len() - 1) as i32) as usize;
            let solution = &mut insertion_ctx.solution;
            let route_ctx = &mut solution.routes.get(route_index).unwrap().clone();

            remove_route(solution, route_ctx)
        });

        insertion_ctx
    }
}

/// Removes a few random, close to each other, routes from solution.
pub struct CloseRouteRemoval {}

impl Default for CloseRouteRemoval {
    fn default() -> Self {
        Self {}
    }
}

impl Ruin for CloseRouteRemoval {
    // NOTE clippy's false positive in route_groups_distances loop
    #[allow(clippy::needless_collect)]
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if let Some(route_groups_distances) = group_routes_by_proximity(&insertion_ctx) {
            let random = &insertion_ctx.environment.random;

            let stale_routes = insertion_ctx
                .solution
                .routes
                .iter()
                .enumerate()
                .filter_map(|(idx, route)| if route.is_stale() { Some(idx) } else { None })
                .collect::<Vec<_>>();

            let route_index = if !stale_routes.is_empty() && random.is_head_not_tails() {
                stale_routes[random.uniform_int(0, (stale_routes.len() - 1) as i32) as usize]
            } else {
                random.uniform_int(0, (route_groups_distances.len() - 1) as i32) as usize
            };

            let take_count = random.uniform_int(2, 3) as usize;

            let routes = route_groups_distances[route_index]
                .iter()
                .take(take_count)
                .filter_map(|(idx, _)| insertion_ctx.solution.routes.get(*idx).cloned())
                .collect::<Vec<_>>();

            routes.into_iter().for_each(|mut route_ctx| {
                remove_route(&mut insertion_ctx.solution, &mut route_ctx);
            });
        }

        insertion_ctx
    }
}

fn remove_route(solution: &mut SolutionContext, route_ctx: &mut RouteContext) {
    if solution.locked.is_empty() {
        remove_whole_route(solution, route_ctx);
    } else {
        remove_part_route(solution, route_ctx);
    }
}

fn remove_whole_route(solution: &mut SolutionContext, route_ctx: &RouteContext) {
    solution.routes.retain(|rc| rc != route_ctx);
    solution.registry.free_route(route_ctx);
    solution.required.extend(route_ctx.route.tour.jobs());
}

fn remove_part_route(solution: &mut SolutionContext, route_ctx: &mut RouteContext) {
    let locked = solution.locked.clone();

    let can_remove_full_route = route_ctx.route.tour.jobs().all(|job| !locked.contains(&job));

    if can_remove_full_route {
        remove_whole_route(solution, route_ctx);
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
