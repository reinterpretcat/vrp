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
use core::construction::constraints::{Demand, DemandDimension};
use core::models::common::*;
use core::models::problem::{Job, Multi};
use core::models::solution::{Route, TourActivity};
use core::models::{Problem, Solution};
use std::io::{BufWriter, Write};

type ApiSolution = crate::json::solution::serializer::Solution;
type ApiSchedule = crate::json::solution::serializer::Schedule;
type DomainExtras = core::models::Extras;

pub trait HereSolution<W: Write> {
    fn write_here(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String>;
}

impl<W: Write> HereSolution<W> for Solution {
    fn write_here(&self, problem: &Problem, writer: BufWriter<W>) -> Result<(), String> {
        let solution = create_solution(problem, &self);
        serialize_solution(writer, &solution).map_err(|err| err.to_string())?;
        Ok(())
    }
}

struct Leg {
    pub location: Location,
    pub departure: Timestamp,
    pub statistic: Statistic,
    pub load: MultiDimensionalCapacity,
}

impl Leg {
    fn new(location: Location, departure: Timestamp, load: MultiDimensionalCapacity) -> Self {
        Self { location, departure, statistic: Statistic::default(), load }
    }
}

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
    let load = route.tour.all_activities().fold(MultiDimensionalCapacity::default(), |acc, activity| {
        acc + activity
            .job
            .as_ref()
            .and_then(|job| get_capacity(&job.as_single().dimens, is_multi_dimen).and_then(|d| Some(d.delivery.0)))
            .unwrap_or(MultiDimensionalCapacity::default())
    });

    let actor = route.actor.as_ref();
    let vehicle = actor.vehicle.as_ref();
    let start = route.tour.start().unwrap();

    let mut tour = Tour {
        vehicle_id: vehicle.dimens.get_id().unwrap().clone(),
        type_id: vehicle.dimens.get_value::<String>("type_id").unwrap().to_string(),
        stops: vec![],
        statistic: Statistic::default(),
    };

    tour.stops.push(Stop {
        location: coord_index.get_by_idx(&start.place.location).unwrap().as_vec(),
        time: format_schedule(&start.schedule),
        load: load.as_vec(),
        activities: vec![Activity {
            job_id: "departure".to_string(),
            activity_type: "departure".to_string(),
            location: Option::None,
            time: Option::None,
            job_tag: Option::None,
        }],
    });

    let mut leg = route.tour.all_activities().skip(1).fold(
        Leg::new(start.place.location, start.schedule.departure, load),
        |acc, act| {
            let activity_type = match &act.job {
                Some(job) => job.as_single().dimens.get_value::<String>("type").unwrap().clone(),
                None => "arrival".to_string(),
            };
            let is_break = activity_type == "break";

            let job_tag = act.job.as_ref().and_then(|job| job.as_single().dimens.get_value::<String>("tag").cloned());

            let job_id = match activity_type.as_str() {
                "pickup" | "delivery" => {
                    let single = act.job.as_ref().unwrap().as_single();
                    let id = single.dimens.get_id().cloned();
                    id.unwrap_or_else(|| Multi::roots(&single).unwrap().dimens.get_id().unwrap().clone())
                }
                _ => activity_type.clone(),
            };

            let driving = problem.transport.duration(vehicle.profile, acc.location, act.place.location, acc.departure);
            let arrival = acc.departure + driving;
            let start = act.schedule.arrival.max(act.place.time.start);
            let waiting = start - act.schedule.arrival;
            let serving = problem.activity.duration(route.actor.as_ref(), act, act.schedule.arrival);
            let departure = start + serving;

            if acc.location != act.place.location {
                tour.stops.push(Stop {
                    location: coord_index.get_by_idx(&act.place.location).unwrap().as_vec(),
                    time: format_as_schedule(&(arrival, departure)),
                    load: acc.load.as_vec(),
                    activities: vec![],
                });
            }

            let load = calculate_load(acc.load, act, is_multi_dimen);

            let last = tour.stops.len() - 1;
            let mut last = tour.stops.get_mut(last).unwrap();

            last.time.departure = format_time(departure);
            last.load = load.as_vec();
            last.activities.push(Activity {
                job_id,
                activity_type,
                location: Some(coord_index.get_by_idx(&act.place.location).unwrap().as_vec()),
                time: Some(Interval { start: format_time(arrival), end: format_time(departure) }),
                job_tag,
            });

            let cost = problem.activity.cost(actor, act, act.schedule.arrival)
                + problem.transport.cost(actor, acc.location, act.place.location, acc.departure);

            let distance =
                problem.transport.distance(vehicle.profile, acc.location, act.place.location, acc.departure) as i32;

            Leg {
                location: act.place.location,
                departure: act.schedule.departure,
                statistic: Statistic {
                    cost: acc.statistic.cost + cost,
                    distance: acc.statistic.distance + distance,
                    duration: acc.statistic.duration + departure as i32 - acc.departure as i32,
                    times: Timing {
                        driving: acc.statistic.times.driving + driving as i32,
                        serving: acc.statistic.times.serving + (if is_break { 0 } else { serving as i32 }),
                        waiting: acc.statistic.times.waiting + waiting as i32,
                        break_time: acc.statistic.times.break_time + (if is_break { serving as i32 } else { 0 }),
                    },
                },
                load,
            }
        },
    );

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
    let job = act.job.as_ref().and_then(|job| Some(job.as_single()));
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

fn create_extras(solution: &Solution) -> Extras {
    if solution.extras.get("iterations").is_some() {
        unimplemented!()
    }

    Extras { performance: vec![] }
}
