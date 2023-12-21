#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/metrics_test.rs"]
mod metrics_test;

use crate::construction::heuristics::{InsertionContext, RouteContext, StateKey};
use crate::models::problem::{TransportCost, TravelTime};
use crate::models::CoreStateKeys;
use rosomaxa::algorithms::math::*;
use rosomaxa::prelude::*;
use std::cmp::Ordering;

/// Gets max load variance in tours.
pub fn get_max_load_variance(insertion_ctx: &InsertionContext) -> f64 {
    let max_load_key = if let Some(capacity_keys) = insertion_ctx.problem.extras.get_capacity_keys() {
        capacity_keys.max_load
    } else {
        return 0.;
    };

    get_variance(get_values_from_route_state(insertion_ctx, max_load_key).collect::<Vec<_>>().as_slice())
}

/// Gets standard deviation of the number of customer per tour.
pub fn get_customers_deviation(insertion_ctx: &InsertionContext) -> f64 {
    let values = insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| route_ctx.route().tour.job_count() as f64)
        .collect::<Vec<_>>();

    get_stdev(values.as_slice())
}

/// Gets mean of route durations.
pub fn get_duration_mean(insertion_ctx: &InsertionContext) -> f64 {
    let total_duration_key = if let Some(schedule_keys) = insertion_ctx.problem.extras.get_schedule_keys() {
        schedule_keys.total_duration
    } else {
        return 0.;
    };

    get_mean_iter(get_values_from_route_state(insertion_ctx, total_duration_key))
}

/// Gets mean of route distances.
pub fn get_distance_mean(insertion_ctx: &InsertionContext) -> f64 {
    let total_distance_key = if let Some(schedule_keys) = insertion_ctx.problem.extras.get_schedule_keys() {
        schedule_keys.total_distance
    } else {
        return 0.;
    };

    get_mean_iter(get_values_from_route_state(insertion_ctx, total_distance_key))
}

/// Gets mean of future waiting time.
pub fn get_waiting_mean(insertion_ctx: &InsertionContext) -> f64 {
    let waiting_time_key = if let Some(schedule_keys) = insertion_ctx.problem.extras.get_schedule_keys() {
        schedule_keys.waiting_time
    } else {
        return 0.;
    };
    get_mean_iter(
        insertion_ctx
            .solution
            .routes
            .iter()
            .filter(|route_ctx| route_ctx.route().tour.get(1).is_some())
            .map(|route_ctx| route_ctx.state().get_activity_state::<f64>(waiting_time_key, 1).cloned().unwrap_or(0.)),
    )
}
/// Gets longest distance between two connected customers (mean, S2).
pub fn get_longest_distance_between_customers_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();
    get_mean_iter(insertion_ctx.solution.routes.iter().map(|route_ctx| {
        route_ctx.route().tour.legs().fold(0., |acc, (activities, _)| match activities {
            [_] => acc,
            [prev, next] => transport
                .distance(
                    route_ctx.route(),
                    prev.place.location,
                    next.place.location,
                    TravelTime::Departure(prev.schedule.departure),
                )
                .max(acc),
            _ => panic!("Unexpected route leg configuration."),
        })
    }))
}

/// Gets average distance between depot to directly-connected customers (mean, S3).
pub fn get_average_distance_between_depot_customer_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();

    get_mean_iter(insertion_ctx.solution.routes.iter().map(|route_ctx| {
        let depot = route_ctx.route().tour.start().expect("empty tour");

        get_mean_iter(route_ctx.route().tour.all_activities().skip(1).map(|activity| {
            transport.distance(
                route_ctx.route(),
                depot.place.location,
                activity.place.location,
                TravelTime::Departure(depot.schedule.departure),
            )
        }))
    }))
}

/// Gets the largest distance between a customer on the route and the depot (mean, S3).
pub fn get_longest_distance_between_depot_customer_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();

    get_mean_iter(insertion_ctx.solution.routes.iter().map(|route_ctx| {
        let depot = route_ctx.route().tour.start().expect("empty tour");

        route_ctx
            .route()
            .tour
            .all_activities()
            .skip(1)
            .map(|activity| {
                transport.distance(
                    route_ctx.route(),
                    depot.place.location,
                    activity.place.location,
                    TravelTime::Departure(depot.schedule.departure),
                )
            })
            .max_by(compare_floats_refs)
            .unwrap_or(0.)
    }))
}

/// Gets average distance between routes using medoids (S4).
pub fn get_distance_gravity_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();
    let profile = insertion_ctx.solution.routes.first().map(|route_ctx| &route_ctx.route().actor.vehicle.profile);

    if let Some(profile) = profile {
        let medoids = insertion_ctx
            .solution
            .routes
            .iter()
            .filter_map(|route_ctx| get_medoid(route_ctx, transport))
            .collect::<Vec<_>>();

        let mut distances = Vec::with_capacity(medoids.len() * 2);

        for i in 0..medoids.len() {
            for j in (i + 1)..medoids.len() {
                let distance = transport.distance_approx(profile, medoids[i], medoids[j]);
                // NOTE assume that negative distance is used between unroutable locations
                distances.push(distance.max(0.));
            }
        }

        get_mean_slice(distances.as_slice())
    } else {
        0.
    }
}

/// Gets medoid location of given route context.
pub fn get_medoid(route_ctx: &RouteContext, transport: &(dyn TransportCost + Send + Sync)) -> Option<usize> {
    let profile = &route_ctx.route().actor.vehicle.profile;
    let locations = route_ctx.route().tour.all_activities().map(|activity| activity.place.location).collect::<Vec<_>>();
    locations
        .iter()
        .map(|outer_loc| {
            let sum = locations
                .iter()
                .map(|inner_loc| transport.distance_approx(profile, *outer_loc, *inner_loc))
                .sum::<f64>();
            (sum, *outer_loc)
        })
        .min_by(|(sum_a, _), (sum_b, _)| compare_floats(*sum_a, *sum_b))
        .map(|(_, location)| location)
}

/// A type which represents routes grouped by their proximity.
pub type RouteProximityGroup = Option<Vec<Vec<(usize, Option<f64>)>>>;

/// Estimates distances between all routes using their medoids and returns the sorted groups.
pub fn group_routes_by_proximity(insertion_ctx: &InsertionContext) -> RouteProximityGroup {
    let solution = &insertion_ctx.solution;
    let profile = &solution.routes.first().map(|route_ctx| &route_ctx.route().actor.vehicle.profile)?;
    let transport = insertion_ctx.problem.transport.as_ref();

    let indexed_medoids = solution
        .routes
        .iter()
        .enumerate()
        .map(|(idx, route_ctx)| (idx, get_medoid(route_ctx, transport)))
        .collect::<Vec<_>>();

    Some(
        indexed_medoids
            .iter()
            .map(|(outer_idx, outer_medoid)| {
                let mut route_distances = indexed_medoids
                    .iter()
                    .filter(move |(inner_idx, _)| *outer_idx != *inner_idx)
                    .map(move |(inner_idx, inner_medoid)| {
                        let distance = match (outer_medoid, inner_medoid) {
                            (Some(outer_medoid), Some(inner_medoid)) => {
                                let distance = transport.distance_approx(profile, *outer_medoid, *inner_medoid);
                                if distance < 0. {
                                    None
                                } else {
                                    Some(distance)
                                }
                            }
                            _ => None,
                        };
                        (*inner_idx, distance)
                    })
                    .collect::<Vec<_>>();

                route_distances.sort_by(|(_, a_distance), (_, b_distance)| match (a_distance, b_distance) {
                    (Some(a_distance), Some(b_distance)) => compare_floats(*a_distance, *b_distance),
                    (Some(_), None) => Ordering::Less,
                    _ => Ordering::Greater,
                });

                route_distances
            })
            .collect::<Vec<_>>(),
    )
}

fn get_values_from_route_state(
    insertion_ctx: &InsertionContext,
    state_key: StateKey,
) -> impl Iterator<Item = f64> + '_ {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(move |route_ctx| route_ctx.state().get_route_state::<f64>(state_key).cloned().unwrap_or(0.))
}
