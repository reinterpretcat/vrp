#[cfg(test)]
#[path = "../../../tests/unit/json/solution/writer_test.rs"]
mod writer_test;

use crate::json::coord_index::CoordIndex;
use crate::json::solution::serializer::Timing;
use crate::json::solution::{serialize_solution, Activity, Extras, Interval, Statistic, Stop, Tour, UnassignedJob};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use core::construction::constraints::{Demand, DemandDimension};
use core::models::common::*;
use core::models::solution::{Route, TourActivity};
use core::models::{Problem, Solution};
use std::io::{BufWriter, Write};
use std::ops::Deref;

type ApiSolution = crate::json::solution::serializer::Solution;
type ApiSchedule = crate::json::solution::serializer::Schedule;

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
    pub load: i32,
}

impl Leg {
    fn new(location: Location, departure: Timestamp, load: i32) -> Self {
        Self { location, departure, statistic: Statistic::default(), load }
    }
}

fn create_solution(problem: &Problem, solution: &Solution) -> ApiSolution {
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

    let unassigned = create_unassigned(problem, solution);

    let extras = create_extras(problem, solution);

    ApiSolution { problem_id, statistic, tours, unassigned, extras }
}

fn create_tour(problem: &Problem, route: &Route, coord_index: &CoordIndex) -> Tour {
    let load = route.tour.all_activities().fold(0, |acc, activity| {
        acc + activity
            .job
            .as_ref()
            .and_then(|job| job.as_single().dimens.get_demand().and_then(|d: &Demand<i32>| Some(d.delivery.0)))
            .unwrap_or(0)
    });

    let vehicle = &route.actor.vehicle;
    let detail = vehicle.details.first().unwrap();
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
        load: vec![load],
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
                "pickup" | "delivery" => act.job.as_ref().unwrap().as_single().dimens.get_id().unwrap().clone(),
                _ => activity_type.clone(),
            };

            let driving = problem.transport.duration(vehicle.profile, acc.location, act.place.location, acc.departure);
            let arrival = acc.departure + driving;
            let start = act.schedule.arrival.max(act.place.time.start);
            let waiting = start - act.schedule.arrival;
            let serving = problem.activity.duration(vehicle, &route.actor.driver, act, act.schedule.arrival);
            let departure = start + serving;

            if acc.location != act.place.location {
                tour.stops.push(Stop {
                    location: coord_index.get_by_idx(&act.place.location).unwrap().as_vec(),
                    time: format_as_schedule(&(arrival, departure)),
                    load: vec![acc.load],
                    activities: vec![],
                });
            }

            let demand = calculate_load(acc.load, act);

            let last = tour.stops.len() - 1;
            let mut last = tour.stops.get_mut(last).unwrap();
            let add_optional_fields = last.activities.len() > 1;

            last.time.departure = format_time(departure);
            last.load[0] = load;
            last.activities.push(Activity {
                job_id,
                activity_type,
                location: if add_optional_fields {
                    Some(coord_index.get_by_idx(&act.place.location).unwrap().as_vec())
                } else {
                    None
                },
                time: if add_optional_fields {
                    Some(Interval { start: format_time(arrival), end: format_time(departure) })
                } else {
                    None
                },
                job_tag,
            });

            let cost = problem.activity.cost(vehicle, &route.actor.driver, act, act.schedule.arrival)
                + problem.transport.cost(vehicle, &route.actor.driver, acc.location, act.place.location, acc.departure);

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

fn calculate_load(current: i32, act: &TourActivity) -> i32 {
    let job = act.job.as_ref().and_then(|job| Some(job.as_single()));
    let demand = job.as_ref().and_then(|job| job.dimens.get_demand().cloned()).unwrap_or(Demand::<i32>::default());
    current - demand.delivery.0 - demand.delivery.1 + demand.pickup.0 + demand.pickup.1
}

fn create_unassigned(problem: &Problem, solution: &Solution) -> Vec<UnassignedJob> {
    unimplemented!()
}

fn create_extras(problem: &Problem, solution: &Solution) -> Extras {
    unimplemented!()
}
