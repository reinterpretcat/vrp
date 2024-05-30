//! Provides the way to control fleet usage.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/fleet_usage_test.rs"]
mod fleet_usage_test;

use super::*;
use std::cmp::Ordering;

/// Creates a feature to minimize used fleet size (affects amount of tours in solution).
pub fn create_minimize_tours_feature(name: &str) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| if route_ctx.route().tour.job_count() == 0 { 1. } else { 0. }),
            solution_estimate_fn: Box::new(|solution_ctx| solution_ctx.routes.iter().len() as Cost),
        })
        .build()
}

/// Creates a feature to maximize used fleet size (affects amount of tours in solution).
pub fn create_maximize_tours_feature(name: &str) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| if route_ctx.route().tour.job_count() == 0 { -1. } else { 0. }),
            solution_estimate_fn: Box::new(|solution_ctx| -1. * solution_ctx.routes.iter().len() as Cost),
        })
        .build()
}

/// Creates a feature to tries to minimize arrival time of used fleet.
pub fn create_minimize_arrival_time_feature(name: &str) -> Result<Feature, GenericError> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| route_ctx.route().actor.detail.time.start),
            solution_estimate_fn: Box::new(|solution_ctx| {
                if solution_ctx.routes.is_empty() {
                    0.
                } else {
                    let total: f64 = solution_ctx
                        .routes
                        .iter()
                        .filter_map(|route_ctx| route_ctx.route().tour.end())
                        .map(|end| end.schedule.arrival)
                        .sum();

                    total / solution_ctx.routes.len() as f64
                }
            }),
        })
        .build()
}

struct FleetUsageObjective {
    route_estimate_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>,
    solution_estimate_fn: Box<dyn Fn(&SolutionContext) -> Cost + Send + Sync>,
}

impl Objective for FleetUsageObjective {
    type Fitness = FitnessContext;
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        self.fitness(a).cmp(&self.fitness(b))
    }

    fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
        (self.fitness(a) - self.fitness(b)).abs()
    }

    fn fitness(&self, solution: &Self::Solution) -> Self::Fitness {
        FitnessContext::Single((self.solution_estimate_fn)(&solution.solution))
    }
}

impl FeatureObjective for FleetUsageObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => (self.route_estimate_fn)(route_ctx),
            _ => Cost::default(),
        }
    }
}
