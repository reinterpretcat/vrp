//! Provides the way to control fleet usage.

use super::*;
use std::ops::Deref;

/// Creates a feature to minimize used fleet size (affects amount of tours in solution).
pub fn create_minimize_tours() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(get_minimization_estimate),
            solution_estimate_fn: Box::new(|solution_ctx| {
                solution_ctx.routes.iter().map(get_minimization_estimate).sum::<f64>()
            }),
        })
        .build()
}

/// Creates a feature to maximize used fleet size (affects amount of tours in solution).
pub fn create_maximize_tours() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(get_maximization_estimate),
            solution_estimate_fn: Box::new(|solution_ctx| {
                solution_ctx.routes.iter().map(get_maximization_estimate).sum::<f64>()
            }),
        })
        .build()
}

/// Creates a feature to tries to minimize arrival time of used fleet.
pub fn create_minimize_arrival_time() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| route_ctx.route.actor.detail.time.start),
            solution_estimate_fn: Box::new(|solution_ctx| {
                if solution_ctx.routes.is_empty() {
                    0.
                } else {
                    let total: f64 = solution_ctx
                        .routes
                        .iter()
                        .filter_map(|route_ctx| route_ctx.route.tour.end())
                        .map(|end| end.schedule.arrival)
                        .sum();

                    total / solution_ctx.routes.len() as f64
                }
            }),
        })
        .build()
}

fn get_minimization_estimate(route_ctx: &RouteContext) -> Cost {
    if route_ctx.route.tour.job_count() == 0 {
        -1.
    } else {
        0.
    }
}

fn get_maximization_estimate(route_ctx: &RouteContext) -> Cost {
    if route_ctx.route.tour.job_count() == 0 {
        1.
    } else {
        0.
    }
}

struct FleetUsageObjective {
    route_estimate_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>,
    solution_estimate_fn: Box<dyn Fn(&SolutionContext) -> Cost + Send + Sync>,
}

impl Objective for FleetUsageObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        self.solution_estimate_fn.deref()(&solution.solution)
    }
}

impl FeatureObjective for FleetUsageObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => self.route_estimate_fn.deref()(route_ctx),
            _ => Cost::default(),
        }
    }
}
