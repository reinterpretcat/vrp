#[cfg(test)]
#[path = "../../../tests/unit/json/solution/writer_test.rs"]
mod writer_test;

use crate::json::coord_index::CoordIndex;
use crate::json::solution::serializer::Timing;
use crate::json::solution::{serialize_solution, Activity, Schedule, Statistic, Stop, Tour};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use core::construction::constraints::{Demand, DemandDimension};
use core::models::common::*;
use core::models::solution::Route;
use core::models::{Problem, Solution};
use std::io::{BufWriter, Write};

type ApiSolution = crate::json::solution::serializer::Solution;

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
    pub distance: Distance,
    pub duration: Duration,
    pub timing: Timing,
    pub cost: Cost,
    pub load: i32,
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
        .unwrap_or_else(|| panic!("Cannot get problem id!"));

    let tours = solution.routes.iter().map(|r| create_tour(r, coord_index)).collect::<Vec<Tour>>();

    let statistic = tours.iter().fold(Statistic::default(), |acc, tour| acc + tour.statistic.clone());

    unimplemented!()
}

fn create_tour(route: &Route, coord_index: &CoordIndex) -> Tour {
    let load = route.tour.all_activities().fold(0, |acc, activity| {
        acc + activity
            .job
            .as_ref()
            .and_then(|job| job.as_single().dimens.get_demand().and_then(|d: &Demand<i32>| Some(d.delivery.0)))
            .unwrap_or(0)
    });

    let vehicle = &route.actor.vehicle;
    let detail = vehicle.details.first().unwrap();

    let mut tour = Tour {
        vehicle_id: vehicle.dimens.get_id().unwrap().clone(),
        type_id: vehicle.dimens.get_value::<String>("type_id").unwrap().to_string(),
        stops: vec![],
        statistic: Statistic::default(),
    };

    tour.stops.push(Stop {
        location: coord_index.get_by_idx(&detail.start.unwrap()).unwrap().as_vec(),
        time: to_schedule(detail.time.as_ref().unwrap()),
        load: vec![load],
        activities: vec![Activity {
            job_id: "departure".to_string(),
            activity_type: "departure".to_string(),
            location: Option::None,
            time: Option::None,
            job_tag: Option::None,
        }],
    });

    tour
}

fn format_time(time: f64) -> String {
    Utc.timestamp(time as i64, 0).to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn to_schedule(tw: &TimeWindow) -> Schedule {
    Schedule { arrival: format_time(tw.start), departure: format_time(tw.end) }
}
