#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/features_test.rs"]
mod features_test;

use crate::construction::enablers::{ScheduleActivityState, TotalDistanceTourState, TotalDurationTourState};
use crate::construction::features::MaxVehicleLoadTourState;
use crate::construction::heuristics::InsertionContext;
use crate::models::common::{Distance, Location};
use crate::models::problem::TravelTime;
use rosomaxa::prelude::*;
use rosomaxa::utils::{ParallelismPolicy, SelectionSamplingIterator, parallel_collect};
use std::cmp::Ordering;
use tinyvec::TinyVec;

/// Features used to position a solution in the Rosomaxa population.
pub(crate) struct RosomaxaSolutionFeatures {
    max_load_variance: Float,
    max_load_mean: Float,
    full_load_ratio: Float,
    duration_mean: Float,
    waiting_mean: Float,
    distance_mean: Float,
    longest_distance_between_customers_mean: Float,
    first_distance_customer_mean: Float,
    last_distance_customer_mean: Float,
    average_distance_between_depot_customer_mean: Float,
    longest_distance_between_depot_customer_mean: Float,
    customers_deviation: Float,
    unassigned_count: Float,
    route_count: Float,
    total_cost: Float,
}

impl From<RosomaxaSolutionFeatures> for Vec<Float> {
    fn from(features: RosomaxaSolutionFeatures) -> Self {
        vec![
            // load related features
            features.max_load_variance,
            features.max_load_mean,
            features.full_load_ratio,
            // time related features
            features.duration_mean,
            features.waiting_mean,
            // distance related features
            features.distance_mean,
            features.longest_distance_between_customers_mean,
            features.first_distance_customer_mean,
            features.last_distance_customer_mean,
            // depot related features
            features.average_distance_between_depot_customer_mean,
            features.longest_distance_between_depot_customer_mean,
            // tour related features
            features.customers_deviation,
            // default objective related features
            features.unassigned_count,
            features.route_count,
            features.total_cost,
        ]
    }
}

#[derive(Default)]
struct Mean {
    sum: Float,
    count: usize,
}

impl Mean {
    fn add(&mut self, value: Float) {
        self.sum += value;
        self.count += 1;
    }

    fn value(&self) -> Float {
        if self.count == 0 { 0. } else { self.sum / self.count as Float }
    }
}

/// Extracts all solution features used by Rosomaxa in two route passes.
///
/// Distance features share the same tour traversal and depot-to-activity distance calculation.
/// The second pass preserves the existing two-pass variance calculation without allocating
/// temporary vectors for route loads and customer counts.
pub(crate) fn get_rosomaxa_solution_features(insertion_ctx: &InsertionContext) -> RosomaxaSolutionFeatures {
    let routes = &insertion_ctx.solution.routes;
    let transport = insertion_ctx.problem.transport.as_ref();
    let route_count = routes.len();

    let mut max_load = Mean::default();
    let mut full_load_count = 0;
    let mut duration = Mean::default();
    let mut waiting = Mean::default();
    let mut distance = Mean::default();
    let mut longest_customer_distance = Mean::default();
    let mut first_customer_distance = Mean::default();
    let mut last_customer_distance = Mean::default();
    let mut average_depot_customer_distance = Mean::default();
    let mut longest_depot_customer_distance = Mean::default();
    let mut customer_count = Mean::default();

    for route_ctx in routes {
        let route = route_ctx.route();
        let state = route_ctx.state();
        let route_max_load = state.get_max_vehicle_load().copied().unwrap_or_default();

        max_load.add(route_max_load);
        full_load_count += usize::from(route_max_load > 0.9);
        duration.add(state.get_total_duration().copied().unwrap_or_default());
        distance.add(state.get_total_distance().copied().unwrap_or_default());
        customer_count.add(route.tour.job_count() as Float);

        if route.tour.get(1).is_some() {
            waiting.add(state.get_schedule_at(1).map(|state| state.waiting_time).unwrap_or_default());
        }

        let route_longest_customer_distance = route.tour.legs().fold(0., |acc, (activities, _)| match activities {
            [_] => acc,
            [prev, next] => transport
                .distance(
                    route,
                    prev.place.location,
                    next.place.location,
                    TravelTime::Departure(prev.schedule.departure),
                )
                .max(acc),
            _ => panic!("Unexpected route leg configuration."),
        });
        longest_customer_distance.add(route_longest_customer_distance);

        let tour_size = route.tour.total();
        if tour_size >= 2 {
            let last_idx = tour_size - 1;
            let before_last_idx = last_idx - 1;

            if let Some((activity, depot)) = route.tour.get(before_last_idx).zip(route.tour.get(last_idx)) {
                last_customer_distance.add(transport.distance(
                    route,
                    activity.place.location,
                    depot.place.location,
                    TravelTime::Departure(activity.schedule.departure),
                ));
            }
        }

        let depot = route.tour.start().expect("empty tour");
        let mut route_depot_customer_distance = Mean::default();
        let mut route_longest_depot_customer_distance = None;

        for activity in route.tour.all_activities().skip(1) {
            let distance = transport.distance(
                route,
                depot.place.location,
                activity.place.location,
                TravelTime::Departure(depot.schedule.departure),
            );

            if route_depot_customer_distance.count == 0 {
                first_customer_distance.add(distance);
            }
            route_depot_customer_distance.add(distance);
            route_longest_depot_customer_distance =
                Some(route_longest_depot_customer_distance.map_or(distance, |longest: Float| {
                    if longest.total_cmp(&distance).is_gt() { longest } else { distance }
                }));
        }

        average_depot_customer_distance.add(route_depot_customer_distance.value());
        if let Some(distance) = route_longest_depot_customer_distance {
            longest_depot_customer_distance.add(distance);
        }
    }

    let max_load_mean = max_load.value();
    let customer_count_mean = customer_count.value();
    let (max_load_first, max_load_second, customer_count_first, customer_count_second) =
        routes.iter().fold((0., 0., 0., 0.), |acc, route_ctx| {
            let max_load = route_ctx.state().get_max_vehicle_load().copied().unwrap_or_default();
            let max_load_deviation = max_load - max_load_mean;
            let customer_count_deviation = route_ctx.route().tour.job_count() as Float - customer_count_mean;

            (
                acc.0 + max_load_deviation * max_load_deviation,
                acc.1 + max_load_deviation,
                acc.2 + customer_count_deviation * customer_count_deviation,
                acc.3 + customer_count_deviation,
            )
        });
    let variance = |first: Float, second: Float| {
        if route_count == 0 { 0. } else { (first - second * second / route_count as Float) / route_count as Float }
    };

    RosomaxaSolutionFeatures {
        max_load_variance: variance(max_load_first, max_load_second),
        max_load_mean,
        full_load_ratio: if route_count == 0 { 0. } else { full_load_count as Float / route_count as Float },
        duration_mean: duration.value(),
        waiting_mean: waiting.value(),
        distance_mean: distance.value(),
        longest_distance_between_customers_mean: longest_customer_distance.value(),
        first_distance_customer_mean: first_customer_distance.value(),
        last_distance_customer_mean: last_customer_distance.value(),
        average_distance_between_depot_customer_mean: average_depot_customer_distance.value(),
        longest_distance_between_depot_customer_mean: longest_depot_customer_distance.value(),
        customers_deviation: variance(customer_count_first, customer_count_second).sqrt(),
        unassigned_count: insertion_ctx.solution.unassigned.len() as Float,
        route_count: route_count as Float,
        total_cost: insertion_ctx.get_total_cost().unwrap_or_default(),
    }
}

/// Estimates distances between all routes by sampling locations from routes and measuring
/// average distance between them.
pub fn group_routes_by_proximity(insertion_ctx: &InsertionContext) -> Vec<Vec<usize>> {
    const LOCATION_SAMPLE_SIZE: usize = 4;

    let routes = &insertion_ctx.solution.routes;
    let transport = insertion_ctx.problem.transport.as_ref();
    let random = &insertion_ctx.environment.random;

    // get routes with sampled locations and index them
    let indexed_route_clusters = routes
        .iter()
        .map(|route_ctx| {
            SelectionSamplingIterator::new(
                route_ctx.route().tour.all_activities(),
                LOCATION_SAMPLE_SIZE,
                random.clone(),
            )
            .map(|activity| activity.place.location)
            .collect::<TinyVec<[Location; LOCATION_SAMPLE_SIZE]>>()
        })
        .enumerate()
        .collect::<Vec<_>>();

    parallel_collect(&indexed_route_clusters, ParallelismPolicy::Default, |(outer_idx, outer_clusters)| {
        let mut route_distances = Vec::with_capacity(indexed_route_clusters.len().saturating_sub(1));

        indexed_route_clusters.iter().for_each(|(inner_idx, inner_clusters)| {
            if *outer_idx != *inner_idx {
                let inner_profile = &routes[*inner_idx].route().actor.vehicle.profile;
                let outer_profile = &routes[*outer_idx].route().actor.vehicle.profile;

                // get a sum of distances between all pairs of sampled locations
                let pair_distance = outer_clusters
                    .iter()
                    .flat_map(|outer| inner_clusters.iter().map(move |inner| (inner, outer)))
                    .map(|(&o, &i)| {
                        // NOTE use outer and inner route profiles to estimate distance
                        let inner_distance = transport.distance_approx(inner_profile, o, i).max(0.);
                        let outer_distance = if inner_profile.index == outer_profile.index {
                            inner_distance
                        } else {
                            transport.distance_approx(outer_profile, o, i).max(0.)
                        };

                        inner_distance + outer_distance
                    })
                    .sum::<Distance>()
                    / 2.;

                let total_pairs = outer_clusters.len() * inner_clusters.len();
                let distance = if total_pairs == 0 {
                    None
                } else {
                    // get average distance between clusters
                    Some(pair_distance / total_pairs as Float)
                };

                route_distances.push((*inner_idx, distance));
            }
        });

        route_distances.sort_unstable_by(|(_, a_distance), (_, b_distance)| match (a_distance, b_distance) {
            (Some(a_distance), Some(b_distance)) => a_distance.total_cmp(b_distance),
            (Some(_), None) => Ordering::Less,
            _ => Ordering::Greater,
        });

        route_distances.into_iter().map(|(index, _)| index).collect()
    })
}
