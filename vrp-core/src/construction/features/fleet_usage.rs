//! Provides the way to control fleet usage.

use super::*;
use std::ops::Deref;

/// Creates a feature to minimize used fleet size (affects amount of tours in solution).
pub fn create_minimize_tours() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(Arc::new(FleetUsageObjective {
            extra_cost_fn: Box::new(|route_ctx| if route_ctx.route.tour.job_count() == 0 { 1. } else { 0. }),
        }))
        .build()
}

/// Creates a feature to maximize used fleet size (affects amount of tours in solution).
pub fn create_maximize_tours() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(Arc::new(FleetUsageObjective {
            extra_cost_fn: Box::new(|route_ctx| if route_ctx.route.tour.job_count() == 0 { -1. } else { 0. }),
        }))
        .build()
}

/// Creates a feature to prefer early starting actors.
pub fn create_early_actor_preference() -> Result<Feature, String> {
    FeatureBuilder::default()
        .with_objective(Arc::new(FleetUsageObjective {
            extra_cost_fn: Box::new(|route_ctx| route_ctx.route.actor.detail.time.start),
        }))
        .build()
}

struct FleetUsageObjective {
    extra_cost_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>,
}

impl Objective for FleetUsageObjective {
    type Solution = InsertionContext;

    fn fitness(&self, solution: &Self::Solution) -> f64 {
        solution.solution.routes.iter().map(self.extra_cost_fn.deref()).sum::<f64>()
    }
}

impl FeatureObjective for FleetUsageObjective {
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => self.extra_cost_fn.deref()(route_ctx),
            _ => 0.,
        }
    }
}
