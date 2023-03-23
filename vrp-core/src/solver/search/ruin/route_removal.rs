#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/route_removal_test.rs"]
mod route_removal_test;

use super::*;
use crate::solver::search::JobRemovalTracker;
use crate::solver::RefinementContext;
use rand::prelude::SliceRandom;

/// A ruin strategy which removes random route from solution.
pub struct RandomRouteRemoval {
    limits: RemovalLimits,
}

impl RandomRouteRemoval {
    /// Creates a new instance of `RandomRouteRemoval`.
    pub fn new(limits: RemovalLimits) -> Self {
        Self { limits }
    }
}

impl Ruin for RandomRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let random = insertion_ctx.environment.random.clone();
        let affected = self.limits.affected_routes_range.end.min(insertion_ctx.solution.routes.len());
        let mut tracker = JobRemovalTracker::new(&self.limits, random.as_ref());

        (0..affected).for_each(|_| {
            let route_index = random.uniform_int(0, (insertion_ctx.solution.routes.len() - 1) as i32) as usize;
            let solution = &mut insertion_ctx.solution;
            let route_ctx = &mut solution.routes.get(route_index).unwrap().clone();

            tracker.try_remove_route(solution, route_ctx, random.as_ref());
        });

        insertion_ctx
    }
}

/// Removes a few random, close to each other, routes from solution.
pub struct CloseRouteRemoval {
    limits: RemovalLimits,
}

impl CloseRouteRemoval {
    /// Creates a new instance of `CloseRouteRemoval`.
    pub fn new(limits: RemovalLimits) -> Self {
        assert!(limits.affected_routes_range.start > 1);
        Self { limits }
    }
}

impl Ruin for CloseRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

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

            #[allow(clippy::needless_collect)]
            let routes = route_groups_distances[route_index]
                .iter()
                .filter_map(|(idx, _)| insertion_ctx.solution.routes.get(*idx).cloned())
                .collect::<Vec<_>>();

            remove_routes(&mut insertion_ctx.solution, &self.limits, random.as_ref(), routes.into_iter());
        }

        insertion_ctx
    }
}

/// Removes a "worst" routes: e.g. the smallest ones.
pub struct WorstRouteRemoval {
    limits: RemovalLimits,
}

impl WorstRouteRemoval {
    /// Creates a new instance of `WorstRouteRemoval`.
    pub fn new(limits: RemovalLimits) -> Self {
        Self { limits }
    }
}

impl Ruin for WorstRouteRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

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

        #[allow(clippy::needless_collect)]
        let routes = route_sizes
            .iter()
            .filter_map(|(idx, _)| insertion_ctx.solution.routes.get(*idx).cloned())
            .collect::<Vec<_>>();

        remove_routes(&mut insertion_ctx.solution, &self.limits, random.as_ref(), routes.into_iter());

        insertion_ctx
    }
}

fn remove_routes<Iter>(
    solution_ctx: &mut SolutionContext,
    limits: &RemovalLimits,
    random: &(dyn Random + Send + Sync),
    routes: Iter,
) where
    Iter: Iterator<Item = RouteContext>,
{
    let mut tracker = JobRemovalTracker::new(limits, random);
    routes.for_each(|mut route_ctx| {
        tracker.try_remove_route(solution_ctx, &mut route_ctx, random);
    });
}
