use vrp_pragmatic::format::problem::*;
use vrp_pragmatic::format::Location;

pub fn create_empty_job() -> Job {
    Job {
        id: "".to_string(),
        pickups: None,
        deliveries: None,
        replacements: None,
        services: None,
        priority: None,
        skills: None,
        value: None,
    }
}

pub fn create_empty_job_task() -> JobTask {
    JobTask { places: vec![], demand: None, tag: None }
}

pub fn create_empty_job_place() -> JobPlace {
    JobPlace { location: Location::Coordinate { lat: 0.0, lng: 0.0 }, duration: 0.0, times: None }
}

pub fn create_test_vehicle_type() -> VehicleType {
    VehicleType {
        type_id: "vehicle".to_string(),
        vehicle_ids: vec!["vehicle_1".to_string()],
        profile: "car".to_string(),
        costs: VehicleCosts { fixed: None, distance: 1., time: 0. },
        shifts: vec![VehicleShift {
            start: ShiftStart {
                earliest: "2020-05-01T09:00:00.00Z".to_string(),
                latest: None,
                location: Location::Coordinate { lat: 0.0, lng: 0.0 },
            },
            end: None,
            dispatch: None,
            breaks: None,
            reloads: None,
        }],
        capacity: vec![10],
        skills: None,
        limits: None,
    }
}

pub fn create_test_vehicle_profile() -> Profile {
    Profile { name: "car".to_string(), profile_type: "car".to_string(), speed: None }
}

pub fn create_test_time_window() -> Vec<String> {
    vec!["2020-07-04T19:00:00.00Z".to_string(), "2020-07-04T21:00:00.00Z".to_string()]
}

pub fn create_test_job(lat: f64, lng: f64) -> Job {
    Job {
        pickups: Some(vec![JobTask {
            places: vec![JobPlace {
                location: Location::Coordinate { lat, lng },
                times: Some(vec![create_test_time_window()]),
                ..create_empty_job_place()
            }],
            demand: Some(vec![1]),
            ..create_empty_job_task()
        }]),
        ..create_empty_job()
    }
}
