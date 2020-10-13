use crate::format::solution::{Activity, Schedule, Solution, Stop, Tour};
use crate::helpers::ToLocation;
use std::cmp::Ordering::{Equal, Less};
use std::collections::HashMap;

pub fn create_stop_with_activity(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: i32,
    time: (&str, &str),
    distance: i64,
) -> Stop {
    create_stop_with_activity_impl(id, activity_type, location, vec![load], time, distance, None)
}

pub fn create_stop_with_activity_md(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: Vec<i32>,
    time: (&str, &str),
    distance: i64,
) -> Stop {
    create_stop_with_activity_impl(id, activity_type, location, load, time, distance, None)
}

pub fn create_stop_with_activity_with_tag(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: i32,
    time: (&str, &str),
    distance: i64,
    job_tag: &str,
) -> Stop {
    create_stop_with_activity_impl(id, activity_type, location, vec![load], time, distance, Some(job_tag.to_string()))
}

fn create_stop_with_activity_impl(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: Vec<i32>,
    time: (&str, &str),
    distance: i64,
    job_tag: Option<String>,
) -> Stop {
    Stop {
        location: vec![location.0, location.1].to_loc(),
        time: Schedule { arrival: time.0.to_string(), departure: time.1.to_string() },
        load,
        distance,
        activities: vec![Activity {
            job_id: id.to_string(),
            activity_type: activity_type.to_string(),
            location: None,
            time: None,
            job_tag,
        }],
    }
}

pub fn assert_vehicle_agnostic(result: Solution, expected: Solution) {
    let mut result = result;

    let tour_map = expected.tours.iter().fold(HashMap::new(), |mut acc, tour| {
        acc.insert(tour.stops.get(1).unwrap().activities.first().unwrap().job_id.clone(), tour.vehicle_id.clone());

        acc
    });

    result.tours.iter_mut().for_each(|tour| {
        let job_id = tour.stops.get(1).unwrap().activities.first().unwrap().job_id.clone();
        if let Some(vehicle_id) = tour_map.get(&job_id) {
            tour.vehicle_id = vehicle_id.to_string();
        }
    });

    result.tours.sort_by(|a, b| {
        let ordering = a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less);

        if ordering == Equal {
            a.shift_index.cmp(&b.shift_index)
        } else {
            ordering
        }
    });

    assert_eq!(result, expected);
}

pub fn create_empty_tour() -> Tour {
    Tour {
        vehicle_id: "".to_string(),
        type_id: "".to_string(),
        shift_index: 0,
        stops: vec![],
        statistic: Default::default(),
    }
}

pub fn create_empty_solution() -> Solution {
    Solution { statistic: Default::default(), tours: vec![], unassigned: None, violations: None, extras: None }
}

pub fn get_ids_from_tour(tour: &Tour) -> Vec<Vec<String>> {
    tour.stops.iter().map(|stop| stop.activities.iter().map(|a| a.job_id.clone()).collect()).collect()
}
