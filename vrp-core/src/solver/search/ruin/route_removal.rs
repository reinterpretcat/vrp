#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/route_removal_test.rs"]
mod route_removal_test;

use super::*;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::*;

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

            remove_route(solution, route_ctx, random.as_ref())
        });

        insertion_ctx
    }
}

/// Removes a few random, close to each other, routes from solution.
#[derive(Default)]
pub struct CloseRouteRemoval {}

impl Ruin for CloseRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if let Some(route_groups_distances) = group_routes_by_proximity(&insertion_ctx) {
            let random = insertion_ctx.environment.random.clone();

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

            #[allow(clippy::needless_collect)]
            let routes = route_groups_distances[route_index]
                .iter()
                .take(take_count)
                .filter_map(|(idx, _)| insertion_ctx.solution.routes.get(*idx).cloned())
                .collect::<Vec<_>>();

            routes.into_iter().for_each(|mut route_ctx| {
                remove_route(&mut insertion_ctx.solution, &mut route_ctx, random.as_ref());
            });
        }

        insertion_ctx
    }
}

/// Removes a "worst" routes: e.g. the smallest ones.
#[derive(Default)]
pub struct WorstRouteRemoval {}

impl Ruin for WorstRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let random = insertion_ctx.environment.random.clone();

        let mut route_sizes = insertion_ctx
            .solution
            .routes
            .iter()
            .enumerate()
            // TODO exclude locked jobs from calculation
            .map(|(route_idx, route_ctx)| (route_idx, route_ctx.route.tour.job_count()))
            .collect::<Vec<_>>();
        route_sizes.sort_by(|(_, job_count_left), (_, job_count_right)| job_count_left.cmp(job_count_right));
        route_sizes.truncate(8);

        let shuffle_amount = (route_sizes.len() as f64 * 0.25) as usize;
        route_sizes.partial_shuffle(&mut random.get_rng(), shuffle_amount);

        let remove_count = if random.is_hit(0.2) { 2 } else { 1 }.min(route_sizes.len());

        #[allow(clippy::needless_collect)]
        let routes = route_sizes
            .iter()
            .take(remove_count)
            .filter_map(|(idx, _)| insertion_ctx.solution.routes.get(*idx).cloned())
            .collect::<Vec<_>>();

        routes.into_iter().for_each(|mut route_ctx| {
            remove_route(&mut insertion_ctx.solution, &mut route_ctx, random.as_ref());
        });

        insertion_ctx
    }
}

fn remove_route(solution: &mut SolutionContext, route_ctx: &mut RouteContext, random: &(dyn Random + Send + Sync)) {
    if can_remove_full_route(solution, route_ctx, random) {
        remove_whole_route(solution, route_ctx);
    } else {
        remove_part_route(solution, route_ctx, random);
    }
}

fn remove_whole_route(solution: &mut SolutionContext, route_ctx: &RouteContext) {
    solution.routes.retain(|rc| rc != route_ctx);
    solution.registry.free_route(route_ctx);
    solution.required.extend(route_ctx.route.tour.jobs());
}

fn remove_part_route(
    solution: &mut SolutionContext,
    route_ctx: &mut RouteContext,
    random: &(dyn Random + Send + Sync),
) {
    const JOB_ACTIVITY_THRESHOLD: usize = 16;

    let locked = solution.locked.clone();

    let mut jobs: Vec<Job> = route_ctx.route.tour.jobs().filter(|job| !locked.contains(job)).collect();

    jobs.shuffle(&mut random.get_rng());
    jobs.truncate(JOB_ACTIVITY_THRESHOLD);

    jobs.iter().for_each(|job| {
        route_ctx.route_mut().tour.remove(job);
    });
    solution.required.extend(jobs);
}

fn can_remove_full_route(
    solution: &SolutionContext,
    route_ctx: &mut RouteContext,
    random: &(dyn Random + Send + Sync),
) -> bool {
    const JOB_ACTIVITY_THRESHOLD: usize = 24;
    const ROUTES_THRESHOLD: usize = 4;
    const REMOVAL_PROBABILITY: f64 = 0.25;

    let no_locked_jobs =
        solution.locked.is_empty() || route_ctx.route.tour.jobs().all(|job| !solution.locked.contains(&job));
    let job_activities = route_ctx.route.tour.job_activity_count();

    if job_activities > JOB_ACTIVITY_THRESHOLD || solution.routes.len() < ROUTES_THRESHOLD {
        no_locked_jobs && random.is_hit(REMOVAL_PROBABILITY)
    } else {
        no_locked_jobs
    }
}
