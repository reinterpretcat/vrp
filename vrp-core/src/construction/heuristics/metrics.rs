use super::InsertionContext;
use crate::algorithms::statistics::{get_mean, get_stdev, get_variance};
use crate::construction::constraints::{MAX_LOAD_KEY, TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY, WAITING_KEY};
use crate::utils::compare_floats;

/// Gets max load variance in tours.
pub fn get_max_load_variance(insertion_ctx: &InsertionContext) -> f64 {
    get_variance(get_values_from_route_state(insertion_ctx, MAX_LOAD_KEY).as_slice())
}

/// Gets standard deviation of the number of customer per tour.
pub fn get_customers_deviation(insertion_ctx: &InsertionContext) -> f64 {
    let values =
        insertion_ctx.solution.routes.iter().map(|route| route.route.tour.job_count() as f64).collect::<Vec<_>>();

    get_stdev(values.as_slice())
}

/// Gets mean of route durations.
pub fn get_duration_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean(get_values_from_route_state(insertion_ctx, TOTAL_DURATION_KEY).as_slice())
}

/// Gets mean of route distances.
pub fn get_distance_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean(get_values_from_route_state(insertion_ctx, TOTAL_DISTANCE_KEY).as_slice())
}

/// Gets mean of future waiting time.
pub fn get_waiting_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean(
        insertion_ctx
            .solution
            .routes
            .iter()
            .filter_map(|route_ctx| route_ctx.route.tour.get(1).map(|a| (route_ctx, a)))
            .map(|(route_ctx, activity)| {
                route_ctx.state.get_activity_state::<f64>(WAITING_KEY, activity).cloned().unwrap_or(0.)
            })
            .collect::<Vec<_>>()
            .as_slice(),
    )
}

/// Gets average distance between routes using medioids.
pub fn get_distance_gravity_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();
    let profile = insertion_ctx.solution.routes.first().map(|route_ctx| route_ctx.route.actor.vehicle.profile);

    if let Some(profile) = profile {
        let medioids = insertion_ctx
            .solution
            .routes
            .iter()
            .filter_map(|route_ctx| {
                let locations =
                    route_ctx.route.tour.all_activities().map(|activity| activity.place.location).collect::<Vec<_>>();
                locations
                    .iter()
                    .map(|outer_loc| {
                        let sum = locations
                            .iter()
                            .map(|inner_loc| {
                                transport.distance(
                                    route_ctx.route.actor.vehicle.profile,
                                    *outer_loc,
                                    *inner_loc,
                                    Default::default(),
                                )
                            })
                            .sum::<f64>();
                        (sum, *outer_loc)
                    })
                    .min_by(|(sum_a, _), (sum_b, _)| compare_floats(*sum_a, *sum_b))
            })
            .map(|(_, location)| location)
            .collect::<Vec<_>>();

        let mut distances = Vec::with_capacity(medioids.len() * 2);

        for i in 0..medioids.len() {
            for j in (i + 1)..medioids.len() {
                let distance = transport.distance(profile, medioids[i], medioids[j], Default::default());
                // NOTE assume that negative distance is used between unroutable locations
                distances.push(distance.max(0.));
            }
        }

        get_mean(distances.as_slice())
    } else {
        0.
    }
}

fn get_values_from_route_state(insertion_ctx: &InsertionContext, state_key: i32) -> Vec<f64> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route| route.state.get_route_state::<f64>(state_key).cloned().unwrap_or(0.))
        .collect()
}
