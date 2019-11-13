use crate::json::solution::{Activity, Schedule, Solution, Stop};
use std::cmp::Ordering::Less;
use std::collections::HashMap;

pub fn create_stop_with_activity(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: i32,
    time: (&str, &str),
) -> Stop {
    Stop {
        location: vec![location.0, location.1],
        time: Schedule { arrival: time.0.to_string(), departure: time.1.to_string() },
        load: vec![load],
        activities: vec![Activity {
            job_id: id.to_string(),
            activity_type: activity_type.to_string(),
            location: None,
            time: None,
            job_tag: None,
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

    result.tours.sort_by(|a, b| a.vehicle_id.partial_cmp(&b.vehicle_id).unwrap_or(Less));

    assert_eq!(result, expected);
}
