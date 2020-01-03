#[cfg(test)]
#[path = "../../../tests/unit/json/solution/writer_test.rs"]
mod writer_test;

use crate::extensions::MultiDimensionalCapacity;
use crate::json::coord_index::CoordIndex;
use crate::json::solution::serializer::Timing;
use crate::json::solution::{
    serialize_solution, Activity, Extras, Interval, Statistic, Stop, Tour, UnassignedJob, UnassignedJobReason,
};
use chrono::{SecondsFormat, TimeZone, Utc};
use std::io::{BufWriter, Write};
use vrp_core::construction::constraints::{Demand, DemandDimension};
use vrp_core::models::common::*;
use vrp_core::models::problem::{Job, Multi};
use vrp_core::models::solution::{Route, TourActivity};
use vrp_core::models::{Problem, Solution};

type ApiSolution = crate::json::solution::serializer::Solution;
type ApiSchedule = crate::json::solution::serializer::Schedule;
type DomainExtras = vrp_core::models::Extras;

/// A trait to serialize solution in pragmatic format.
pub trait PragmaticSolution<W: Write> {
    fn write_pragmatic(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> PragmaticSolution<W> for Solution {
    fn write_pragmatic(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String> {
        let solution = create_solution(problem, &self);
        serialize_solution(writer, &solution).map_err(|err| err.to_string())?;
        Ok(())
    }
}

struct Leg {
    pub last_detail: Option<(Location, Timestamp)>,
    pub load: Option<MultiDimensionalCapacity>,
    pub statistic: Statistic,
}

impl Leg {
    fn new(
        last_detail: Option<(Location, Timestamp)>,
        load: Option<MultiDimensionalCapacity>,
        statistic: Statistic,
    ) -> Self {
        Self { last_detail, load, statistic }
    }

    fn empty() -> Self {
        Self { last_detail: None, load: None, statistic: Statistic::default() }
    }
}

/// Creates solution.
pub fn create_solution(problem: &Problem, solution: &Solution) -> ApiSolution {
    let coord_index = solution
        .extras
        .get("coord_index")
        .and_then(|s| s.downcast_ref::<CoordIndex>())
        .unwrap_or_else(|| panic!("Cannot get coord index!"));

    let problem_id = solution
        .extras
        .get("problem_id")
        .and_then(|s| s.downcast_ref::<String>())
        .unwrap_or_else(|| panic!("Cannot get problem id!"))
        .clone();

    let tours = solution.routes.iter().map(|r| create_tour(problem, r, coord_index)).collect::<Vec<Tour>>();

    let statistic = tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

    let unassigned = create_unassigned(solution);

    let extras = create_extras(solution);

    ApiSolution { problem_id, statistic, tours, unassigned, extras }
}

fn create_tour(problem: &Problem, route: &Route, coord_index: &CoordIndex) -> Tour {
    let is_multi_dimen = has_multi_dimensional_capacity(problem.extras.as_ref());

    let actor = route.actor.as_ref();
    let vehicle = actor.vehicle.as_ref();

    let mut tour = Tour {
        vehicle_id: vehicle.dimens.get_id().unwrap().clone(),
        type_id: vehicle.dimens.get_value::<String>("type_id").unwrap().to_string(),
        stops: vec![],
        statistic: Statistic::default(),
    };

    let last_idx = route.tour.total() - 1;
    let mut leg = (0_usize..)
        .zip(route.tour.all_activities())
        .fold(Vec::<(usize, usize)>::default(), |mut acc, (idx, a)| {
            if get_activity_type(&a).map_or(false, |t| t == "reload") || idx == route.tour.total() - 1 {
                let start_idx = acc.last().map_or(0_usize, |item| item.1 + 1);
                let end_idx = if idx == last_idx { last_idx } else { idx - 1 };

                acc.push((start_idx, end_idx));
            }

            acc
        })
        .into_iter()
        .fold(Leg::empty(), |leg, (start_idx, end_idx)| {
            let (start_delivery, end_pickup) = route.tour.activities_slice(start_idx, end_idx).iter().fold(
                (leg.load.unwrap_or_else(|| MultiDimensionalCapacity::default()), MultiDimensionalCapacity::default()),
                |acc, activity| {
                    let (delivery, pickup) = activity
                        .job
                        .as_ref()
                        .and_then(|job| {
                            get_capacity(&job.to_single().dimens, is_multi_dimen)
                                .and_then(|d| Some((d.delivery.0, d.pickup.0)))
                        })
                        .unwrap_or((MultiDimensionalCapacity::default(), MultiDimensionalCapacity::default()));
                    (acc.0 + delivery, acc.1 + pickup)
                },
            );

            let (start_idx, start) = if start_idx == 0 {
                let start = route.tour.start().unwrap();
                tour.stops.push(Stop {
                    location: coord_index.get_by_idx(&start.place.location).unwrap(),
                    time: format_schedule(&start.schedule),
                    load: start_delivery.as_vec(),
                    activities: vec![Activity {
                        job_id: "departure".to_string(),
                        activity_type: "departure".to_string(),
                        location: None,
                        time: None,
                        job_tag: None,
                    }],
                });
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
                        MultiDimensionalCapacity::new(vec![0; dimen_size])
                    };

                    let activity_type = activity_type.unwrap_or_else(|| "arrival".to_string());
                    let is_break = activity_type == "break";

                    let job_tag =
                        act.job.as_ref().and_then(|job| job.to_single().dimens.get_value::<String>("tag").cloned());
                    let job_id = match activity_type.as_str() {
                        "pickup" | "delivery" => {
                            let single = act.job.as_ref().unwrap().to_single();
                            let id = single.dimens.get_id().cloned();
                            id.unwrap_or_else(|| Multi::roots(&single).unwrap().dimens.get_id().unwrap().clone())
                        }
                        _ => activity_type.clone(),
                    };

                    let driving =
                        problem.transport.duration(vehicle.profile, prev_location, act.place.location, prev_departure);
                    let arrival = prev_departure + driving;
                    let start = act.schedule.arrival.max(act.place.time.start);
                    let waiting = start - act.schedule.arrival;
                    let serving = problem.activity.duration(route.actor.as_ref(), act, act.schedule.arrival);
                    let departure = start + serving;

                    if prev_location != act.place.location {
                        tour.stops.push(Stop {
                            location: coord_index.get_by_idx(&act.place.location).unwrap(),
                            time: format_as_schedule(&(arrival, departure)),
                            load: prev_load.as_vec(),
                            activities: vec![],
                        });
                    }

                    let load = calculate_load(prev_load, act, is_multi_dimen);

                    let last = tour.stops.len() - 1;
                    let mut last = tour.stops.get_mut(last).unwrap();

                    last.time.departure = format_time(departure);
                    last.load = load.as_vec();
                    last.activities.push(Activity {
                        job_id,
                        activity_type,
                        location: Some(coord_index.get_by_idx(&act.place.location).unwrap()),
                        time: Some(Interval { start: format_time(arrival), end: format_time(departure) }),
                        job_tag,
                    });

                    let cost = problem.activity.cost(actor, act, act.schedule.arrival)
                        + problem.transport.cost(actor, prev_location, act.place.location, prev_departure);

                    let distance =
                        problem.transport.distance(vehicle.profile, prev_location, act.place.location, prev_departure)
                            as i32;

                    Leg {
                        last_detail: Some((act.place.location, act.schedule.departure)),
                        statistic: Statistic {
                            cost: leg.statistic.cost + cost,
                            distance: leg.statistic.distance + distance,
                            duration: leg.statistic.duration + departure as i32 - prev_departure as i32,
                            times: Timing {
                                driving: leg.statistic.times.driving + driving as i32,
                                serving: leg.statistic.times.serving + (if is_break { 0 } else { serving as i32 }),
                                waiting: leg.statistic.times.waiting + waiting as i32,
                                break_time: leg.statistic.times.break_time
                                    + (if is_break { serving as i32 } else { 0 }),
                            },
                        },
                        load: Some(load),
                    }
                },
            );

            leg.load = Some(leg.load.unwrap() - end_pickup);

            leg
        });

    // NOTE remove redundant info
    tour.stops
        .iter_mut()
        .filter(|stop| stop.activities.len() == 1)
        .flat_map(|stop| stop.activities.iter_mut())
        .for_each(|activity| {
            activity.location = None;
            activity.time = None;
        });

    leg.statistic.cost += vehicle.costs.fixed;

    tour.vehicle_id = vehicle.dimens.get_id().unwrap().clone();
    tour.type_id = vehicle.dimens.get_value::<String>("type_id").unwrap().clone();
    tour.statistic = leg.statistic;

    tour
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn format_schedule(schedule: &Schedule) -> ApiSchedule {
    ApiSchedule { arrival: format_time(schedule.arrival), departure: format_time(schedule.departure) }
}

fn format_as_schedule(schedule: &(f64, f64)) -> ApiSchedule {
    format_schedule(&Schedule::new(schedule.0, schedule.1))
}

fn calculate_load(
    current: MultiDimensionalCapacity,
    act: &TourActivity,
    is_multi_dimen: bool,
) -> MultiDimensionalCapacity {
    let job = act.job.as_ref().and_then(|job| job.as_single());
    let demand = job
        .as_ref()
        .and_then(|job| get_capacity(&job.dimens, is_multi_dimen))
        .unwrap_or(Demand::<MultiDimensionalCapacity>::default());
    current - demand.delivery.0 - demand.delivery.1 + demand.pickup.0 + demand.pickup.1
}

fn create_unassigned(solution: &Solution) -> Vec<UnassignedJob> {
    solution.unassigned.iter().fold(vec![], |mut acc, unassigned| {
        let reason = match unassigned.1 {
            1 => (2, "cannot be visited within time window"),
            2 => (3, "does not fit into any vehicle due to capacity"),
            5 => (101, "cannot be assigned due to max distance constraint of vehicle"),
            6 => (102, "cannot be assigned due to shift time constraint of vehicle"),
            10 => (1, "cannot serve required skill"),
            11 => (100, "location unreachable"),
            _ => (0, "unknown"),
        };
        let dimens = match unassigned.0.as_ref() {
            Job::Single(job) => &job.dimens,
            Job::Multi(job) => &job.dimens,
        };
        acc.push(UnassignedJob {
            job_id: dimens
                .get_value::<String>("vehicle_id")
                .map(|vehicle_id| format!("{}_break", vehicle_id))
                .unwrap_or_else(|| dimens.get_id().unwrap().clone()),
            reasons: vec![UnassignedJobReason { code: reason.0, description: reason.1.to_string() }],
        });

        acc
    })
}

fn get_activity_type(activity: &TourActivity) -> Option<&String> {
    activity.job.as_ref().and_then(|job| job.as_single()).and_then(|single| single.dimens.get_value::<String>("type"))
}

fn get_capacity(dimens: &Dimensions, is_multi_dimen: bool) -> Option<Demand<MultiDimensionalCapacity>> {
    if is_multi_dimen {
        dimens.get_demand().cloned()
    } else {
        let create_capacity = |value: i32| {
            if value == 0 {
                MultiDimensionalCapacity::default()
            } else {
                MultiDimensionalCapacity::new(vec![value])
            }
        };
        dimens.get_demand().map(|demand: &Demand<i32>| Demand {
            pickup: (create_capacity(demand.pickup.0), create_capacity(demand.pickup.1)),
            delivery: (create_capacity(demand.delivery.0), create_capacity(demand.delivery.1)),
        })
    }
}

fn has_multi_dimensional_capacity(extras: &DomainExtras) -> bool {
    let capacity_type = extras
        .get("capacity_type")
        .and_then(|s| s.downcast_ref::<String>())
        .unwrap_or_else(|| panic!("Cannot get capacity type!"));
    match capacity_type.as_str() {
        "multi" => true,
        "single" => false,
        _ => panic!("Unknown capacity type: '{}'", capacity_type),
    }
}

fn create_extras(solution: &Solution) -> Option<Extras> {
    if solution.extras.get("iterations").is_some() {
        unimplemented!()
    }

    None
}
