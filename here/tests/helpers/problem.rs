use crate::helpers::format_time;
use crate::json::problem::{
    Job, JobPlace, JobPlaces, JobVariant, Matrix, VehicleCosts, VehiclePlace, VehiclePlaces, VehicleType,
};

pub fn create_delivery_job(id: &str, location: Vec<f64>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces { pickup: Option::None, delivery: Some(create_job_place(location)) },
        demand: vec![1],
        skills: None,
    })
}

pub fn create_delivery_job_with_duration(id: &str, location: Vec<f64>, duration: f64) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces {
            pickup: Option::None,
            delivery: Some(JobPlace { times: None, location, duration, tag: None }),
        },
        demand: vec![1],
        skills: None,
    })
}

pub fn create_delivery_job_with_skills(id: &str, location: Vec<f64>, skills: Vec<String>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces {
            pickup: Option::None,
            delivery: Some(JobPlace { times: None, location, duration: 1., tag: None }),
        },
        demand: vec![1],
        skills: Some(skills),
    })
}

pub fn create_delivery_job_with_times(
    id: &str,
    location: Vec<f64>,
    times: Vec<(i32, i32)>,
    duration: f64,
) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces {
            pickup: Option::None,
            delivery: Some(JobPlace {
                times: Some(times.iter().map(|tw| vec![format_time(tw.0), format_time(tw.1)]).collect()),
                location,
                duration,
                tag: None,
            }),
        },
        demand: vec![1],
        skills: None,
    })
}

pub fn create_pickup_job(id: &str, location: Vec<f64>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces { pickup: Some(create_job_place(location)), delivery: Option::None },
        demand: vec![1],
        skills: None,
    })
}

pub fn create_pickup_delivery_job(id: &str, pickup_location: Vec<f64>, delivery_location: Vec<f64>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces {
            pickup: Some(create_job_place(pickup_location)),
            delivery: Some(create_job_place(delivery_location)),
        },
        demand: vec![1],
        skills: None,
    })
}

fn create_job_place(location: Vec<f64>) -> JobPlace {
    JobPlace { times: None, location, duration: 1., tag: None }
}

pub fn create_default_vehicle_places() -> VehiclePlaces {
    create_default_vehicle_places_with_locations((0., 0.), (0., 0.))
}

pub fn create_default_open_vehicle_places() -> VehiclePlaces {
    VehiclePlaces { start: VehiclePlace { time: format_time(0), location: vec![0., 0.] }, end: None, max_tours: None }
}

pub fn create_default_vehicle_places_with_locations(start: (f64, f64), end: (f64, f64)) -> VehiclePlaces {
    VehiclePlaces {
        start: VehiclePlace { time: format_time(0), location: vec![start.0, start.1] },
        end: Some(VehiclePlace { time: format_time(1000).to_string(), location: vec![end.0, end.1] }),
        max_tours: None,
    }
}

pub fn create_default_vehicle_costs() -> VehicleCosts {
    VehicleCosts { fixed: Some(10.), distance: 1., time: 1. }
}

pub fn create_default_vehicle(id: &str) -> VehicleType {
    VehicleType {
        id: id.to_string(),
        profile: "car".to_string(),
        costs: create_default_vehicle_costs(),
        places: create_default_vehicle_places(),
        capacity: vec![10],
        amount: 1,
        skills: None,
        limits: None,
        vehicle_break: None,
    }
}

pub fn create_matrix(data: Vec<i64>) -> Matrix {
    let size = (data.len() as f64).sqrt() as i32;

    assert_eq!((size * size) as usize, data.len());

    Matrix {
        num_origins: size,
        num_destinations: size,
        travel_times: data.clone(),
        distances: data.clone(),
        error_codes: None,
    }
}

pub fn to_strings(data: Vec<&str>) -> Vec<String> {
    data.iter().map(|item| item.to_string()).collect()
}
