#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/metrics_test.rs"]
mod metrics_test;

use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState, WaitingTimeActivityState};
use crate::construction::features::MaxVehicleLoadTourState;
use crate::construction::heuristics::{InsertionContext, RouteContext, RouteState};
use crate::models::common::Distance;
use crate::models::problem::{TransportCost, TravelTime};
use rosomaxa::algorithms::math::*;
use rosomaxa::prelude::*;
use rosomaxa::utils::parallel_collect;
use std::cmp::Ordering;

/// Gets max load variance in tours.
pub fn get_max_load_variance(insertion_ctx: &InsertionContext) -> Float {
    get_variance(
        get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load()).collect::<Vec<_>>().as_slice(),
    )
}

/// Gets max load mean in tours.
pub fn get_max_load_mean(insertion_ctx: &InsertionContext) -> Float {
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load()))
}

/// Gets tours with max_load at least 0.9.
pub fn get_full_load_ratio(insertion_ctx: &InsertionContext) -> Float {
    let total = insertion_ctx.solution.routes.len();
    if total == 0 {
        0.
    } else {
        let full_capacity = get_values_from_route_state(insertion_ctx, |state| state.get_max_vehicle_load())
            .filter(|&max_load| max_load > 0.9)
            .count();

        full_capacity as Float / total as Float
    }
}

/// Gets standard deviation of the number of customer per tour.
pub fn get_customers_deviation(insertion_ctx: &InsertionContext) -> Float {
    let values = insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| route_ctx.route().tour.job_count() as Float)
        .collect::<Vec<_>>();

    get_stdev(values.as_slice())
}

/// Gets mean of route durations.
pub fn get_duration_mean(insertion_ctx: &InsertionContext) -> Float {
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_total_duration()))
}

/// Gets mean of route distances.
pub fn get_distance_mean(insertion_ctx: &InsertionContext) -> Float {
    get_mean_iter(get_values_from_route_state(insertion_ctx, |state| state.get_total_distance()))
}

/// Gets mean of future waiting time.
pub fn get_waiting_mean(insertion_ctx: &InsertionContext) -> Float {
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
pub fn get_longest_distance_between_customers_mean(insertion_ctx: &InsertionContext) -> Float {
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
pub fn get_average_distance_between_depot_customer_mean(insertion_ctx: &InsertionContext) -> Float {
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
pub fn get_longest_distance_between_depot_customer_mean(insertion_ctx: &InsertionContext) -> Float {
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
pub fn get_first_distance_customer_mean(insertion_ctx: &InsertionContext) -> Float {
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
pub fn get_last_distance_customer_mean(insertion_ctx: &InsertionContext) -> Float {
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

/// Estimates distances between all routes using their medoids and returns the sorted groups.
pub fn group_routes_by_proximity(insertion_ctx: &InsertionContext) -> Option<Vec<Vec<usize>>> {
    let solution = &insertion_ctx.solution;
    let profile = &solution.routes.first().map(|route_ctx| &route_ctx.route().actor.vehicle.profile)?;
    let transport = insertion_ctx.problem.transport.as_ref();

    let indexed_route_clusters =
        parallel_collect(&solution.routes, |route_ctx| get_approx_clusters(route_ctx, transport))
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>();

    Some(parallel_collect(&indexed_route_clusters, |(outer_idx, outer_clusters)| {
        let mut route_distances = indexed_route_clusters
            .iter()
            .filter(move |(inner_idx, _)| *outer_idx != *inner_idx)
            .map(move |(inner_idx, inner_clusters)| {
                let distance = match (outer_clusters, inner_clusters) {
                    (Some(outer_clusters), Some(inner_clusters)) => {
                        // get a sum of distances between all pairs of clusters
                        let pair_distance = outer_clusters
                            .iter()
                            .flat_map(|outer| inner_clusters.iter().map(move |inner| (inner, outer)))
                            .map(|(&o, &i)| {
                                transport.distance_approx(profile, o, i).max(0.)
                                    + transport.distance_approx(profile, i, o).max(0.)
                            })
                            .sum::<Distance>()
                            / 2.;

                        let total_pairs = outer_clusters.len() * inner_clusters.len();
                        if total_pairs == 0 {
                            None
                        } else {
                            // get average distance between clusters
                            Some(pair_distance / total_pairs as Float)
                        }
                    }
                    _ => None,
                };

                (*inner_idx, distance)
            })
            .collect::<Vec<_>>();

        route_distances.sort_unstable_by(|(_, a_distance), (_, b_distance)| match (a_distance, b_distance) {
            (Some(a_distance), Some(b_distance)) => compare_floats(*a_distance, *b_distance),
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Greater,
        });

        let (indices, _): (Vec<_>, Vec<_>) = route_distances.into_iter().unzip();

        indices
    }))
}

fn get_values_from_route_state<'a>(
    insertion_ctx: &'a InsertionContext,
    state_value_fn: impl Fn(&'a RouteState) -> Option<&Float> + 'a,
) -> impl Iterator<Item = Float> + 'a {
    insertion_ctx
        .solution
        .routes
        .iter()
        .map(move |route_ctx| state_value_fn(route_ctx.state()).copied().unwrap_or_default())
}

fn get_approx_clusters(route_ctx: &RouteContext, transport: &(dyn TransportCost)) -> Option<Vec<usize>> {
    const CLUSTER_SIZE_RATIO: Float = 0.2;

    let distance_threshold = route_ctx.state().get_total_distance().copied()? * CLUSTER_SIZE_RATIO;
    let profile = &route_ctx.route().actor.vehicle.profile;

    Some(
        route_ctx
            .route()
            .tour
            .all_activities()
            .map(|activity| activity.place.location)
            .fold((Distance::default(), None, Vec::default()), |(mut acc_distance, prev, mut clusters), current| {
                if let Some(prev) = prev {
                    acc_distance += transport.distance_approx(profile, prev, current).max(0.);
                    if acc_distance > distance_threshold {
                        clusters.push(current);
                        acc_distance = Distance::default();
                    }
                } else {
                    clusters.push(current)
                }

                (acc_distance, Some(current), clusters)
            })
            .2,
    )
}
