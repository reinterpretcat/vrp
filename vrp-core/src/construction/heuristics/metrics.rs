#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/metrics_test.rs"]
mod metrics_test;

use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState, WaitingTimeActivityState};
use crate::construction::features::MaxVehicleLoadTourState;
use crate::construction::heuristics::{InsertionContext, RouteContext, RouteState};
use crate::models::problem::{TransportCost, TravelTime};
use rosomaxa::algorithms::math::*;
use rosomaxa::prelude::*;
use std::cmp::Ordering;

/// Gets max load variance in tours.
pub fn get_max_load_variance(insertion_ctx: &InsertionContext) -> f64 {
    get_variance(
        get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load()).collect::<Vec<_>>().as_slice(),
    )
}

/// Gets max load mean in tours.
pub fn get_max_load_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load()))
}

/// Gets tours with max_load at least 0.9.
pub fn get_full_load_ratio(insertion_ctx: &InsertionContext) -> f64 {
    let total = insertion_ctx.solution.routes.len();
    if total == 0 {
        0.
    } else {
        let full_capacity = get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load())
            .filter(|&max_load| max_load > 0.9)
            .count();

        full_capacity as f64 / total as f64
    }
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
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_total_duration()))
}

/// Gets mean of route distances.
pub fn get_distance_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_total_distance()))
}

/// Gets mean of future waiting time.
pub fn get_waiting_mean(insertion_ctx: &InsertionContext) -> f64 {
    get_mean_iter(
        insertion_ctx
            .solution
            .routes
            .iter()
            .filter(|route_ctx| route_ctx.route().tour.get(1).is_some())
            .map(|route_ctx| route_ctx.state().get_waiting_time_at(1).copied().unwrap_or(0.)),
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

    get_mean_iter(insertion_ctx.solution.routes.iter().filter_map(|route_ctx| {
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
    }))
}

/// Gets the distance between a first job and the depot (mean).
pub fn get_first_distance_customer_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();

    get_mean_iter(insertion_ctx.solution.routes.iter().filter_map(|route_ctx| {
        let route = route_ctx.route();
        route.tour.get(1).zip(route.tour.start()).map(|(activity, depot)| {
            transport.distance(
                route,
                depot.place.location,
                activity.place.location,
                TravelTime::Departure(depot.schedule.departure),
            )
        })
    }))
}

/// Gets the distance between a last job and the depot (mean).
pub fn get_last_distance_customer_mean(insertion_ctx: &InsertionContext) -> f64 {
    let transport = insertion_ctx.problem.transport.as_ref();

    let distances = insertion_ctx.solution.routes.iter().filter_map(|route_ctx| {
        let tour = &route_ctx.route().tour;
        if tour.total() < 2 {
            return None;
        }

        // NOTE if it is open VRP, we get a distance between two last customers
        let last_idx = tour.total() - 1;
        let before_last_idx = last_idx - 1;

        tour.get(before_last_idx).zip(tour.get(last_idx)).map(|(activity, depot)| {
            transport.distance(
                route_ctx.route(),
                activity.place.location,
                depot.place.location,
                TravelTime::Departure(depot.schedule.departure),
            )
        })
    });

    get_mean_iter(distances)
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

fn get_values_from_route_state<'a>(
    insertion_ctx: &'a InsertionContext,
    state_value_fn: impl Fn(&'a RouteState) -> Option<&f64> + 'a,
) -> impl Iterator<Item = f64> + 'a {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(move |route_ctx| state_value_fn(route_ctx.state()).copied().unwrap_or_default())
}

/// Gets medoid location of given route context.
fn get_medoid(route_ctx: &RouteContext, transport: &(dyn TransportCost)) -> Option<usize> {
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
