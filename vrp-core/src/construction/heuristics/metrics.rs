use super::InsertionContext;
use crate::algorithms::statistics::{get_mean, get_stdev, get_variance};
use crate::construction::constraints::{MAX_LOAD_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::models::common::Location;
use std::ops::Deref;
use std::sync::Arc;

/// Resolvers location to two dimensional coordinate, potentially using
/// multidimensional scaling algorithm.
pub struct LocationResolver {
    /// A function which does mapping from location to 2D coordinate.
    pub func: Arc<dyn Fn(Location) -> (f64, f64) + Sync + Send>,
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
pub fn get_distance_gravity_mean(insertion_ctx: &InsertionContext) -> f64 {
    let location_resolver =
        insertion_ctx.problem.extras.get("location_resolver").and_then(|s| s.downcast_ref::<LocationResolver>());

    if let Some(location_resolver) = location_resolver {
        let solution_shape = insertion_ctx
            .solution
            .routes
            .iter()
            .map(|route_ctx| {
                route_ctx.route.tour.all_activities().map(|activity| activity.place.location).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        calculate_gravity_distance_mean(solution_shape.as_slice(), location_resolver)
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

fn calculate_gravity_distance_mean(shape: &[Vec<Location>], location_resolver: &LocationResolver) -> f64 {
    if shape.is_empty() {
        return 0.;
    }

    let centroids = shape
        .iter()
        .map(|polygon| {
            let (sum_x, sum_y) = polygon
                .iter()
                .map(|location| location_resolver.func.deref()(*location))
                .fold((0., 0.), |(sum_x, sum_y), (x, y)| (sum_x + x, sum_y + y));

            (sum_x / polygon.len() as f64, sum_y / polygon.len() as f64)
        })
        .collect::<Vec<_>>();

    let mut distances = Vec::with_capacity(centroids.len() * 2);

    for i in 0..centroids.len() {
        for j in (i + 1)..centroids.len() {
            let (x1, y1) = centroids[i];
            let (x2, y2) = centroids[j];
            distances.push(((x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2)).sqrt());
        }
    }

    get_mean(distances.as_slice())
}
