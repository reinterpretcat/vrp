use crate::helpers::format_time;
use crate::json::problem::{Job, JobPlace, JobPlaces, JobVariant, VehicleCosts, VehiclePlace, VehiclePlaces};

pub fn create_delivery_job(id: &str, location: Vec<f64>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces { pickup: Option::None, delivery: Some(create_job_place(location)) },
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

fn create_job_place(location: Vec<f64>) -> JobPlace {
    JobPlace { times: None, location, duration: 1., tag: None }
}

pub fn create_default_vehicle_places() -> VehiclePlaces {
    create_default_vehicle_places_with_locations((0., 0.), (0., 0.))
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
