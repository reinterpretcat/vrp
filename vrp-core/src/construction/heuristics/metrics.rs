use super::InsertionContext;
use crate::algorithms::statistics::{get_mean, get_stdev, get_variance};
use crate::construction::constraints::{MAX_LOAD_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::models::common::Location;
use std::ops::Deref;
use std::sync::Arc;

/// Specifies a gravity center calculator.
pub struct GravityCalculator {
    func: Arc<dyn Fn(Vec<Vec<Location>>) -> f64>,
}

/// Gets max load variance in tours.
pub fn get_max_load_variance(insertion_ctx: &InsertionContext) -> f64 {
    get_variance(get_values_from_state(insertion_ctx, MAX_LOAD_KEY).as_slice())
}

/// Gets standard deviation of the number of customer per tour.
pub fn get_customers_deviation(insertion_ctx: &InsertionContext) -> f64 {
    let values =
        insertion_ctx.solution.routes.iter().map(|route| route.route.tour.job_count() as f64).collect::<Vec<_>>();

    get_stdev(values.as_slice())
}

/// Gets mean of route durations.
pub fn get_duration_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean(get_values_from_state(insertion_ctx, TOTAL_DURATION_KEY).as_slice())
}

/// Gets mean of route distances.
pub fn get_distance_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean(get_values_from_state(insertion_ctx, TOTAL_DISTANCE_KEY).as_slice())
}

/// Gets average distance between routes (their centers of gravity).
pub fn get_distance_gravity(insertion_ctx: &InsertionContext) -> f64 {
    let gravity_calculator =
        insertion_ctx.problem.extras.get("gravity_calculator").and_then(|s| s.downcast_ref::<GravityCalculator>());

    if let Some(gravity_calculator) = gravity_calculator {
        let solution_shape = insertion_ctx
            .solution
            .routes
            .iter()
            .map(|route_ctx| {
                route_ctx.route.tour.all_activities().map(|activity| activity.place.location).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        gravity_calculator.func.deref()(solution_shape)
    } else {
        0.
    }
}

fn get_values_from_state(insertion_ctx: &InsertionContext, state_key: i32) -> Vec<f64> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route| route.state.get_route_state::<f64>(state_key).cloned().unwrap_or(0.))
        .collect()
}
