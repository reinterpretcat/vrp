use crate::format_time;
use crate::helpers::ToLocation;
use crate::json::coord_index::CoordIndex;
use crate::json::problem::*;
use vrp_core::models::common::{Distance, Duration, Location, Timestamp};
use vrp_core::models::problem::TransportCost;

fn create_job_place(location: Vec<f64>) -> JobPlace {
    JobPlace { times: None, location: location.to_loc(), duration: 1. }
}

fn create_task(location: Vec<f64>) -> JobTask {
    JobTask { places: vec![create_job_place(location)], demand: Some(vec![1]), tag: None }
}

fn create_job(id: &str) -> Job {
    Job {
        id: id.to_string(),
        pickups: None,
        deliveries: None,
        replacements: None,
        services: None,
        priority: None,
        skills: None,
    }
}

pub fn create_delivery_job(id: &str, location: Vec<f64>) -> Job {
    Job { deliveries: Some(vec![create_task(location.clone())]), ..create_job(id) }
}

pub fn create_delivery_job_with_priority(id: &str, location: Vec<f64>, priority: i32) -> Job {
    Job { priority: Some(priority), ..create_delivery_job(id, location) }
}

pub fn create_delivery_job_with_skills(id: &str, location: Vec<f64>, skills: Vec<String>) -> Job {
    Job { skills: Some(skills), ..create_delivery_job(id, location) }
}

pub fn create_delivery_job_with_demand(id: &str, location: Vec<f64>, demand: Vec<i32>) -> Job {
    Job { deliveries: Some(vec![JobTask { demand: Some(demand), ..create_task(location) }]), ..create_job(id) }
}

pub fn create_delivery_job_with_duration(id: &str, location: Vec<f64>, duration: f64) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { duration, ..create_job_place(location) }],
            demand: Some(vec![1]),
            tag: None,
        }]),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_times(id: &str, location: Vec<f64>, times: Vec<(i32, i32)>, duration: f64) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { duration, times: convert_times(&times), ..create_job_place(location) }],
            demand: Some(vec![1]),
            tag: None,
        }]),
        ..create_job(id)
    }
}

pub fn create_pickup_job(id: &str, location: Vec<f64>) -> Job {
    Job { pickups: Some(vec![create_task(location.clone())]), ..create_job(id) }
}

pub fn create_pickup_job_with_demand(id: &str, location: Vec<f64>, demand: Vec<i32>) -> Job {
    Job { pickups: Some(vec![JobTask { demand: Some(demand), ..create_task(location) }]), ..create_job(id) }
}

pub fn create_replacement_job(id: &str, location: Vec<f64>) -> Job {
    Job { replacements: Some(vec![create_task(location.clone())]), ..create_job(id) }
}

pub fn create_service_job(id: &str, location: Vec<f64>) -> Job {
    Job { services: Some(vec![JobTask { demand: None, ..create_task(location.clone()) }]), ..create_job(id) }
}

pub fn create_pickup_delivery_job(id: &str, pickup_location: Vec<f64>, delivery_location: Vec<f64>) -> Job {
    Job {
        pickups: Some(vec![create_task(pickup_location.clone())]),
        deliveries: Some(vec![create_task(delivery_location.clone())]),
        ..create_job(id)
    }
}

pub fn create_pickup_delivery_job_with_params(
    id: &str,
    demand: Vec<i32>,
    pickup: (Vec<f64>, f64, Vec<(i32, i32)>),
    delivery: (Vec<f64>, f64, Vec<(i32, i32)>),
) -> Job {
    Job {
        pickups: Some(vec![JobTask {
            places: vec![JobPlace {
                duration: pickup.1,
                times: convert_times(&pickup.2),
                ..create_job_place(pickup.0.clone())
            }],
            demand: Some(demand.clone()),
            tag: None,
        }]),
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace {
                duration: delivery.1,
                times: convert_times(&delivery.2),
                ..create_job_place(delivery.0.clone())
            }],
            demand: Some(demand.clone()),
            tag: None,
        }]),

        ..create_job(id)
    }
}

pub fn create_multi_job(
    id: &str,
    pickups: Vec<((f64, f64), f64, Vec<i32>)>,
    deliveries: Vec<((f64, f64), f64, Vec<i32>)>,
) -> Job {
    let create_tasks = |tasks: Vec<((f64, f64), f64, Vec<i32>)>| {
        let tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(i, (location, duration, demand))| JobTask {
                places: vec![JobPlace { duration, ..create_job_place(vec![location.0, location.1]) }],
                demand: Some(demand),
                tag: Some((i + 1).to_string()),
            })
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    };

    Job { pickups: create_tasks(pickups), deliveries: create_tasks(deliveries), ..create_job(id) }
}

pub fn create_default_vehicle_shift() -> VehicleShift {
    create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))
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

pub fn create_default_vehicle_type() -> VehicleType {
    create_default_vehicle("my_vehicle")
}

pub fn create_default_vehicle(id: &str) -> VehicleType {
    create_vehicle_with_capacity(id, vec![10])
}

pub fn create_vehicle_with_capacity(id: &str, capacity: Vec<i32>) -> VehicleType {
    VehicleType {
        type_id: id.to_string(),
        vehicle_ids: vec![format!("{}_1", id)],
        profile: "car".to_string(),
        costs: create_default_vehicle_costs(),
        shifts: vec![create_default_vehicle_shift()],
        capacity,
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

    Matrix { travel_times: data.clone(), distances: data.clone(), error_codes: None }
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

fn convert_times(times: &Vec<(i32, i32)>) -> Option<Vec<Vec<String>>> {
    if times.is_empty() {
        None
    } else {
        Some(times.iter().map(|tw| vec![format_time(tw.0 as f64), format_time(tw.1 as f64)]).collect())
    }
}
