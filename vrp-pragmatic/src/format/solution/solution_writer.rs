#[cfg(test)]
#[path = "../../../tests/unit/format/solution/writer_test.rs"]
mod writer_test;

use crate::format::solution::activity_matcher::get_job_tag;
use crate::format::solution::model::Timing;
use crate::format::solution::*;
use crate::format::CoordIndex;
use vrp_core::construction::enablers::{get_route_intervals, ReservedTimesIndex};
use vrp_core::construction::features::JobDemandDimension;
use vrp_core::construction::heuristics::UnassignmentInfo;
use vrp_core::models::common::*;
use vrp_core::models::problem::{JobIdDimension, Multi, TravelTime, VehicleIdDimension};
use vrp_core::models::solution::{Activity, Route};
use vrp_core::prelude::Float;
use vrp_core::rosomaxa::evolution::TelemetryMetrics;
use vrp_core::solver::processing::{ClusterConfigExtraProperty, ReservedTimesExtraProperty};
use vrp_core::utils::CollectGroupBy;

struct Leg {
    pub last_detail: Option<(DomainLocation, Timestamp)>,
    pub load: Option<MultiDimLoad>,
    pub statistic: Statistic,
}

impl Leg {
    fn new(last_detail: Option<(DomainLocation, Timestamp)>, load: Option<MultiDimLoad>, statistic: Statistic) -> Self {
        Self { last_detail, load, statistic }
    }

    fn empty() -> Self {
        Self { last_detail: None, load: None, statistic: Statistic::default() }
    }
}

/// Creates solution.
pub(crate) fn create_solution(
    problem: &DomainProblem,
    solution: &DomainSolution,
    output_type: &PragmaticOutputType,
) -> ApiSolution {
    let coord_index = problem.extras.get_coord_index().expect("no coord index");

    let empty_reserved_times = Default::default();
    let reserved_times_index = problem.extras.get_reserved_times();
    let reserved_times_index = reserved_times_index.as_ref().unwrap_or(&empty_reserved_times);

    let tours = solution
        .routes
        .iter()
        .map(|r| create_tour(problem, r, &coord_index, reserved_times_index))
        .collect::<Vec<Tour>>();

    let statistic = tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

    let unassigned = create_unassigned(solution);
    let violations = create_violations(solution);

    let api_solution = ApiSolution { statistic, tours, unassigned, violations, extras: None };

    let extras = create_extras(problem, &api_solution, solution.telemetry.as_ref(), output_type);

    ApiSolution { extras, ..api_solution }
}

fn create_tour(
    problem: &DomainProblem,
    route: &Route,
    coord_index: &CoordIndex,
    reserved_times_index: &ReservedTimesIndex,
) -> Tour {
    // TODO reduce complexity
    let parking = get_parking_time(problem.extras.as_ref());

    let actor = route.actor.as_ref();
    let vehicle = actor.vehicle.as_ref();
    let transport = problem.transport.as_ref();

    let mut tour = Tour {
        vehicle_id: vehicle.dimens.get_vehicle_id().unwrap().clone(),
        type_id: vehicle.dimens.get_vehicle_type().unwrap().clone(),
        shift_index: vehicle.dimens.get_shift_index().copied().unwrap(),
        stops: vec![],
        statistic: Statistic::default(),
    };

    let intervals = get_route_intervals(route, |a| get_activity_type(a).map_or(false, |t| t == "reload"));

    let mut leg = intervals.into_iter().fold(Leg::empty(), |leg, (start_idx, end_idx)| {
        let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
            (leg.load.unwrap_or_default(), MultiDimLoad::default()),
            |acc, activity| {
                let (delivery, pickup) = activity
                    .job
                    .as_ref()
                    .and_then(|job| get_capacity(&job.dimens).map(|d| (d.delivery.0, d.pickup.0)))
                    .unwrap_or((MultiDimLoad::default(), MultiDimLoad::default()));
                (acc.0 + delivery, acc.1 + pickup)
            },
        );

        let (start_idx, start) = if start_idx == 0 {
            let start = route.tour.start().unwrap();
            let is_same_location =
                route.tour.get(1).map_or(false, |activity| start.place.location == activity.place.location);

            tour.stops.push(Stop::Point(PointStop {
                location: coord_index.get_by_idx(start.place.location).unwrap(),
                time: format_schedule(&start.schedule),
                load: start_delivery.as_vec(),
                distance: 0,
                activities: vec![ApiActivity {
                    job_id: "departure".to_string(),
                    activity_type: "departure".to_string(),
                    location: None,
                    time: if is_same_location {
                        Some(Interval {
                            start: format_time(start.schedule.arrival),
                            end: format_time(start.schedule.departure),
                        })
                    } else {
                        None
                    },
                    job_tag: None,
                    commute: None,
                }],
                parking: None,
            }));
            (start_idx + 1, start)
        } else {
            (start_idx, route.tour.get(start_idx - 1).unwrap())
        };

        let mut leg = route.tour.activities_slice(start_idx, end_idx).iter().fold(
            Leg::new(Some((start.place.location, start.schedule.departure)), Some(start_delivery), leg.statistic),
            |leg, act| {
                let activity_type = get_activity_type(act).cloned();
                let (prev_location, prev_departure) = leg.last_detail.unwrap();
                let prev_load = if activity_type.is_some() {
                    leg.load.unwrap()
                } else {
                    // NOTE arrival must have zero load
                    let dimen_size = leg.load.unwrap().size;
                    MultiDimLoad::new(vec![0; dimen_size])
                };

                let activity_type = activity_type.unwrap_or_else(|| "arrival".to_string());
                let is_break = activity_type == "break";

                let job_tag = act.job.as_ref().and_then(|single| {
                    get_job_tag(single, (act.place.location, (act.place.time.clone(), start.schedule.departure)))
                        .cloned()
                });
                let job_id = match activity_type.as_str() {
                    "pickup" | "delivery" | "replacement" | "service" => {
                        let single = act.job.as_ref().unwrap();
                        let id = single.dimens.get_job_id().cloned();
                        id.unwrap_or_else(|| Multi::roots(single).unwrap().dimens.get_job_id().unwrap().clone())
                    }
                    _ => activity_type.clone(),
                };

                let commute = act.commute.clone().unwrap_or_default();
                let commuting = commute.duration();

                let (driving, transport_cost) = if commute.is_zero_distance() {
                    // NOTE: use original cost traits to adapt time-based costs (except waiting/commuting)
                    let prev_departure = TravelTime::Departure(prev_departure);
                    let duration = transport.duration(route, prev_location, act.place.location, prev_departure);
                    let transport_cost = transport.cost(route, prev_location, act.place.location, prev_departure);
                    (duration, transport_cost)
                } else {
                    // NOTE: no need to drive in case of non-zero commute, this goes to commuting time
                    (0, commuting as Float * vehicle.costs.per_service_time)
                };

                // NOTE two clusters at the same stop location
                let parking =
                    match (prev_location == act.place.location, act.commute.is_some(), commute.is_zero_distance()) {
                        (false, true, true) => parking,
                        _ => 0,
                    };

                let activity_arrival = parking + act.schedule.arrival + commute.forward.duration;
                let service_start = activity_arrival.max(act.place.time.start);
                let waiting = service_start - activity_arrival;
                let serving = act.place.duration - parking;
                let service_end = service_start + serving;
                let activity_departure = service_end;

                // TODO: add better support of time based activity costs
                let serving_cost = problem.activity.cost(route, act, service_start);
                let total_cost = serving_cost + transport_cost + waiting as Float * vehicle.costs.per_waiting_time;

                let location_distance =
                    transport.distance(route, prev_location, act.place.location, TravelTime::Departure(prev_departure));
                let distance = leg.statistic.distance + location_distance - commute.forward.distance;

                let is_new_stop = match (act.commute.as_ref(), prev_location == act.place.location) {
                    (Some(commute), false) if commute.is_zero_distance() => true,
                    (Some(_), _) => false,
                    (None, is_same_location) => !is_same_location,
                };

                if is_new_stop {
                    tour.stops.push(Stop::Point(PointStop {
                        location: coord_index.get_by_idx(act.place.location).unwrap(),
                        time: format_schedule(&act.schedule),
                        load: prev_load.as_vec(),
                        distance,
                        parking: if parking > 0 {
                            Some(Interval {
                                start: format_time(act.schedule.arrival),
                                end: format_time(act.schedule.arrival + parking),
                            })
                        } else {
                            None
                        },
                        activities: vec![],
                    }));
                }

                let load = calculate_load(prev_load, act);

                let last = tour.stops.len() - 1;
                let last = match tour.stops.get_mut(last).unwrap() {
                    Stop::Point(point) => point,
                    Stop::Transit(_) => unreachable!(),
                };

                last.time.departure = format_time(act.schedule.departure);
                last.load = load.as_vec();
                last.activities.push(ApiActivity {
                    job_id,
                    activity_type: activity_type.clone(),
                    location: Some(coord_index.get_by_idx(act.place.location).unwrap()),
                    time: Some(Interval {
                        start: format_time(activity_arrival.max(act.place.time.start)),
                        end: format_time(activity_departure),
                    }),
                    job_tag,
                    commute: act
                        .commute
                        .as_ref()
                        .map(|commute| Commute::new(commute, act.schedule.arrival, activity_departure, coord_index)),
                });

                // NOTE detect when vehicle returns after activity to stop point
                let end_location = if commute.backward.is_zero_distance() {
                    act.place.location
                } else {
                    tour.stops
                        .last()
                        .and_then(|stop| stop.as_point())
                        .and_then(|stop| coord_index.get_by_loc(&stop.location))
                        .expect("expect to have at least one stop")
                };

                Leg {
                    last_detail: Some((end_location, act.schedule.departure)),
                    statistic: Statistic {
                        cost: leg.statistic.cost + total_cost,
                        distance,
                        duration: leg.statistic.duration + act.schedule.departure - prev_departure,
                        times: Timing {
                            driving: leg.statistic.times.driving + driving,
                            serving: leg.statistic.times.serving + (if is_break { 0 } else { serving }),
                            waiting: leg.statistic.times.waiting + waiting,
                            break_time: leg.statistic.times.break_time + (if is_break { serving } else { 0 }),
                            commuting: leg.statistic.times.commuting + commuting,
                            parking: leg.statistic.times.parking + parking,
                        },
                    },
                    load: Some(load),
                }
            },
        );

        leg.load = Some(leg.load.unwrap() - end_pickup);

        leg
    });

    leg.statistic.cost += vehicle.costs.fixed;
    tour.statistic = leg.statistic;

    insert_reserved_times_as_breaks(route, &mut tour, reserved_times_index);

    // NOTE remove redundant info from single activity on the stop
    tour.stops
        .iter_mut()
        .filter(|stop| stop.activities().len() == 1)
        .flat_map(|stop| {
            let schedule = stop.schedule().clone();
            let location = stop.location().cloned();
            stop.activities_mut().first_mut().map(|activity| (location, schedule, activity))
        })
        .for_each(|(location, schedule, activity)| {
            let is_same_schedule = activity.time.as_ref().map_or(true, |time| schedule.arrival == time.start);
            let is_same_location = activity.location.clone().zip(location).map_or(true, |(lhs, rhs)| lhs == rhs);

            if is_same_schedule {
                activity.time = None;
            }

            if is_same_location {
                activity.location = None;
            }
        });

    tour.vehicle_id.clone_from(vehicle.dimens.get_vehicle_id().unwrap());
    tour.type_id.clone_from(vehicle.dimens.get_vehicle_type().unwrap());

    tour
}

fn format_schedule(schedule: &DomainSchedule) -> ApiSchedule {
    ApiSchedule { arrival: format_time(schedule.arrival), departure: format_time(schedule.departure) }
}

fn calculate_load(current: MultiDimLoad, act: &Activity) -> MultiDimLoad {
    let job = act.job.as_ref();
    let demand = job.and_then(|job| get_capacity(&job.dimens)).unwrap_or_default();
    current - demand.delivery.0 - demand.delivery.1 + demand.pickup.0 + demand.pickup.1
}

fn create_unassigned(solution: &DomainSolution) -> Option<Vec<UnassignedJob>> {
    let create_simple_reasons = |code: ViolationCode| {
        let (code, reason) = map_code_reason(code);
        vec![UnassignedJobReason { code: code.to_string(), description: reason.to_string(), details: None }]
    };

    let unassigned = solution
        .unassigned
        .iter()
        .filter(|(job, _)| job.dimens().get_vehicle_id().is_none())
        .map(|(job, code)| {
            let job_id = job.dimens().get_job_id().expect("job id expected").clone();

            let reasons = match code {
                UnassignmentInfo::Simple(code) => create_simple_reasons(*code),
                UnassignmentInfo::Detailed(details) if !details.is_empty() => details
                    .iter()
                    .collect_group_by_key(|(_, code)| *code)
                    .into_iter()
                    .map(|(code, group)| {
                        let (code, reason) = map_code_reason(code);
                        let mut vehicle_details = group
                            .iter()
                            .map(|(actor, _)| {
                                let dimens = &actor.vehicle.dimens;
                                let vehicle_id = dimens.get_vehicle_id().cloned().unwrap();
                                let shift_index = dimens.get_shift_index().copied().unwrap();
                                (vehicle_id, shift_index)
                            })
                            .collect::<Vec<_>>();
                        // NOTE sort to have consistent order
                        vehicle_details.sort();

                        UnassignedJobReason {
                            details: Some(
                                vehicle_details
                                    .into_iter()
                                    .map(|(vehicle_id, shift_index)| UnassignedJobDetail { vehicle_id, shift_index })
                                    .collect(),
                            ),
                            code: code.to_string(),
                            description: reason.to_string(),
                        }
                    })
                    .collect(),
                _ => create_simple_reasons(ViolationCode(0)),
            };

            UnassignedJob { job_id, reasons }
        })
        .collect::<Vec<_>>();

    if unassigned.is_empty() {
        None
    } else {
        Some(unassigned)
    }
}

fn create_violations(solution: &DomainSolution) -> Option<Vec<Violation>> {
    // NOTE at the moment only break violation is mapped
    let violations = solution
        .unassigned
        .iter()
        .filter(|(job, _)| job.dimens().get_job_type().map_or(false, |t| t == "break"))
        .map(|(job, _)| Violation::Break {
            vehicle_id: job.dimens().get_vehicle_id().expect("vehicle id").clone(),
            shift_index: job.dimens().get_shift_index().copied().expect("shift index"),
        })
        .collect::<Vec<_>>();

    if violations.is_empty() {
        None
    } else {
        Some(violations)
    }
}

fn get_activity_type(activity: &Activity) -> Option<&String> {
    activity.job.as_ref().and_then(|single| single.dimens.get_job_type())
}

fn get_capacity(dimens: &Dimensions) -> Option<Demand<MultiDimLoad>> {
    // NOTE: try to detect whether dimensions stores multidimensional demand
    let demand: Option<Demand<MultiDimLoad>> = dimens.get_job_demand().cloned();
    if let Some(demand) = demand {
        return Some(demand);
    }

    let create_capacity = |capacity: SingleDimLoad| {
        if capacity.value == 0 {
            MultiDimLoad::default()
        } else {
            MultiDimLoad::new(vec![capacity.value])
        }
    };
    dimens.get_job_demand().map(|demand: &Demand<SingleDimLoad>| Demand {
        pickup: (create_capacity(demand.pickup.0), create_capacity(demand.pickup.1)),
        delivery: (create_capacity(demand.delivery.0), create_capacity(demand.delivery.1)),
    })
}

fn get_parking_time(extras: &DomainExtras) -> Duration {
    extras.get_cluster_config().map_or(0, |config| config.serving.get_parking())
}

fn create_extras(
    problem: &DomainProblem,
    solution: &ApiSolution,
    metrics: Option<&TelemetryMetrics>,
    output_type: &PragmaticOutputType,
) -> Option<Extras> {
    match output_type {
        PragmaticOutputType::OnlyPragmatic => {
            get_api_metrics(metrics).map(|metrics| Extras { metrics: Some(metrics), features: None })
        }
        PragmaticOutputType::OnlyGeoJson => None,
        PragmaticOutputType::Combined => {
            Some(Extras {
                metrics: get_api_metrics(metrics),
                // TODO do not hide error here, propagate it to the caller
                features: create_feature_collection(problem, solution).ok(),
            })
        }
    }
}

fn get_api_metrics(metrics: Option<&TelemetryMetrics>) -> Option<ApiMetrics> {
    metrics.as_ref().map(|metrics| ApiMetrics {
        duration: metrics.duration,
        generations: metrics.generations,
        speed: metrics.speed,
        evolution: metrics
            .evolution
            .iter()
            .map(|g| ApiGeneration {
                number: g.number,
                timestamp: g.timestamp,
                i_all_ratio: g.i_all_ratio,
                i_1000_ratio: g.i_1000_ratio,
                is_improvement: g.is_improvement,
                population: AppPopulation {
                    individuals: g
                        .population
                        .individuals
                        .iter()
                        .map(|i| ApiIndividual { difference: i.difference, fitness: i.fitness.clone() })
                        .collect(),
                },
            })
            .collect(),
    })
}
