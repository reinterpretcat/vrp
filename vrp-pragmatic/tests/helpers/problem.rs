use crate::format_time;
use crate::helpers::ToLocation;
use crate::json::coord_index::CoordIndex;
use crate::json::problem::*;
use vrp_core::models::common::{Distance, Duration, Location, Timestamp};
use vrp_core::models::problem::TransportCost;

pub fn create_delivery_job(id: &str, location: Vec<f64>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces { pickup: Option::None, delivery: Some(create_job_place(location)) },
        demand: vec![1],
        skills: None,
    })
}

pub fn create_delivery_job_with_demand(id: &str, location: Vec<f64>, demand: Vec<i32>) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces { pickup: Option::None, delivery: Some(create_job_place(location)) },
        demand,
        skills: None,
    })
}

pub fn create_delivery_job_with_duration(id: &str, location: Vec<f64>, duration: f64) -> JobVariant {
    JobVariant::Single(Job {
        id: id.to_string(),
        places: JobPlaces {
            pickup: Option::None,
            delivery: Some(JobPlace { times: None, location: location.to_loc(), duration, tag: None }),
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
            delivery: Some(JobPlace { times: None, location: location.to_loc(), duration: 1., tag: None }),
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
                times: Some(times.iter().map(|tw| vec![format_time(tw.0 as f64), format_time(tw.1 as f64)]).collect()),
                location: location.to_loc(),
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

pub fn create_multi_job(
    id: &str,
    pickups: Vec<((f64, f64), f64, Vec<i32>)>,
    deliveries: Vec<((f64, f64), f64, Vec<i32>)>,
) -> JobVariant {
    JobVariant::Multi(MultiJob {
        id: id.to_string(),
        places: MultiJobPlaces {
            pickups: pickups
                .into_iter()
                .enumerate()
                .map(|(i, (location, duration, demand))| MultiJobPlace {
                    times: Option::None,
                    location: vec![location.0, location.1].to_loc(),
                    duration,
                    demand,
                    tag: Some((i + 1).to_string()),
                })
                .collect(),
            deliveries: deliveries
                .into_iter()
                .enumerate()
                .map(|(i, (location, duration, demand))| MultiJobPlace {
                    times: Option::None,
                    location: vec![location.0, location.1].to_loc(),
                    duration,
                    demand,
                    tag: Some((i + 1).to_string()),
                })
                .collect(),
        },
        skills: Option::None,
    })
}

fn create_job_place(location: Vec<f64>) -> JobPlace {
    JobPlace { times: None, location: location.to_loc(), duration: 1., tag: None }
}

pub fn create_default_vehicle_shift() -> VehicleShift {
    create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))
}

pub fn create_default_vehicle_shift_with_breaks(breaks: Vec<VehicleBreak>) -> VehicleShift {
    VehicleShift {
        start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
        end: Some(VehiclePlace { time: format_time(1000.).to_string(), location: vec![0., 0.].to_loc() }),
        breaks: Some(breaks),
        reloads: None,
    }
}

pub fn create_default_open_vehicle_shift() -> VehicleShift {
    VehicleShift {
        start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
        end: None,
        breaks: None,
        reloads: None,
    }
}

pub fn create_default_vehicle_shift_with_locations(start: (f64, f64), end: (f64, f64)) -> VehicleShift {
    VehicleShift {
        start: VehiclePlace { time: format_time(0.), location: vec![start.0, start.1].to_loc() },
        end: Some(VehiclePlace { time: format_time(1000.).to_string(), location: vec![end.0, end.1].to_loc() }),
        breaks: None,
        reloads: None,
    }
}

pub fn create_default_vehicle_costs() -> VehicleCosts {
    VehicleCosts { fixed: Some(10.), distance: 1., time: 1. }
}

pub fn create_default_vehicle(id: &str) -> VehicleType {
    create_vehicle_with_capacity(id, vec![10])
}

pub fn create_vehicle_with_capacity(id: &str, capacity: Vec<i32>) -> VehicleType {
    VehicleType {
        id: id.to_string(),
        profile: "car".to_string(),
        costs: create_default_vehicle_costs(),
        shifts: vec![create_default_vehicle_shift()],
        capacity,
        amount: 1,
        skills: None,
        limits: None,
    }
}

pub fn create_default_profiles() -> Vec<Profile> {
    vec![Profile { name: "car".to_string(), profile_type: "car".to_string() }]
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

pub fn create_matrix_from_problem(problem: &Problem) -> Matrix {
    let unique = CoordIndex::new(problem).unique();

    let data: Vec<i64> = unique
        .iter()
        .cloned()
        .flat_map(|a| {
            unique.iter().map(move |b| ((a.lat - b.lat).powf(2.) + (a.lng - b.lng).powf(2.)).sqrt().round() as i64)
        })
        .collect();

    create_matrix(data)
}

pub fn to_strings(data: Vec<&str>) -> Vec<String> {
    data.iter().map(|item| item.to_string()).collect()
}

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration(&self, _profile: i32, from: Location, to: Location, _departure: Timestamp) -> Duration {
        (if to > from { to - from } else { from - to }) as f64
    }

    fn distance(&self, _profile: i32, _from: Location, _to: Location, _departure: Timestamp) -> Distance {
        unimplemented!()
    }
}

impl TestTransportCost {
    pub fn new() -> Self {
        Self {}
    }
}
