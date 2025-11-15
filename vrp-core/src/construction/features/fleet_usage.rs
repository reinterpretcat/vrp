//! Provides the way to control fleet usage.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/fleet_usage_test.rs"]
mod fleet_usage_test;

use std::collections::HashMap;

use super::*;

/// Creates a feature to minimize used fleet size (affects amount of tours in solution).
pub fn create_minimize_tours_feature(name: &str) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| if route_ctx.route().tour.job_count() == 0 { 1. } else { 0. }),
            solution_estimate_fn: Box::new(|solution_ctx| solution_ctx.routes.iter().len() as Cost),
        })
        .build()
}

/// Creates a feature to maximize used fleet size (affects amount of tours in solution).
pub fn create_maximize_tours_feature(name: &str) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| if route_ctx.route().tour.job_count() == 0 { -1. } else { 0. }),
            solution_estimate_fn: Box::new(|solution_ctx| -(solution_ctx.routes.iter().len() as Cost)),
        })
        .build()
}

/// Creates a feature to tries to minimize arrival time of used fleet.
pub fn create_minimize_arrival_time_feature(name: &str) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|route_ctx| route_ctx.route().actor.detail.time.start),
            solution_estimate_fn: Box::new(|solution_ctx| {
                if solution_ctx.routes.is_empty() {
                    0.
                } else {
                    let total: Float = solution_ctx
                        .routes
                        .iter()
                        .filter_map(|route_ctx| route_ctx.route().tour.end())
                        .map(|end| end.schedule.arrival)
                        .sum();

                    total / solution_ctx.routes.len() as Float
                }
            }),
        })
        .build()
}

/// Creates a feature to distribute shifts evenly across vehicles.
/// This encourages using different shifts from different vehicles rather than
/// exhausting all shifts from one vehicle before using another.
pub fn create_balance_shifts_feature(name: &str) -> GenericResult<Feature> {
    FeatureBuilder::default()
        .with_name(name)
        .with_objective(FleetUsageObjective {
            route_estimate_fn: Box::new(|_| 0.),
            solution_estimate_fn: Box::new(|solution_ctx| {
                if solution_ctx.routes.is_empty() {
                    return 0.;
                }

                // Group routes by vehicle ID
                let mut vehicle_shift_counts: HashMap<String, usize> = HashMap::new();
                let mut total_available_shifts: HashMap<String, usize> = HashMap::new();

                // Count how many shifts each vehicle is using
                for route_ctx in solution_ctx.routes.iter() {
                    let actor = &route_ctx.route().actor;
                    if let Some(vehicle_id) = actor.vehicle.dimens.get_vehicle_id() {
                        *vehicle_shift_counts.entry(vehicle_id.clone()).or_insert(0) += 1;

                        // Track total available shifts for this vehicle
                        total_available_shifts.entry(vehicle_id.clone()).or_insert(actor.vehicle.details.len());
                    }
                }

                // Calculate the imbalance score
                // We want to minimize the variance of (used_shifts / available_shifts) ratio
                if vehicle_shift_counts.is_empty() {
                    return 0.;
                }

                let ratios: Vec<f64> = vehicle_shift_counts
                    .iter()
                    .map(|(vehicle_id, &used_count)| {
                        let available = *total_available_shifts.get(vehicle_id).unwrap_or(&1) as f64;
                        used_count as f64 / available
                    })
                    .collect();

                // Calculate variance of ratios
                let mean: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
                let variance: f64 = ratios
                    .iter()
                    .map(|&ratio| {
                        let diff = ratio - mean;
                        diff * diff
                    })
                    .sum::<f64>()
                    / ratios.len() as f64;

                variance
            }),
        })
        .build()
}

struct FleetUsageObjective {
    route_estimate_fn: Box<dyn Fn(&RouteContext) -> Cost + Send + Sync>,
    solution_estimate_fn: Box<dyn Fn(&SolutionContext) -> Cost + Send + Sync>,
}

impl FeatureObjective for FleetUsageObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        (self.solution_estimate_fn)(&solution.solution)
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, .. } => (self.route_estimate_fn)(route_ctx),
            _ => Cost::default(),
        }
    }
}
