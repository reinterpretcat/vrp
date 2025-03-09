use crate::format::problem::Objective::{MinimizeCost, MinimizeUnassigned};
use crate::format::problem::*;
use crate::format::{CoordIndex, Location};
use crate::format_time;
use crate::helpers::ToLocation;
use vrp_core::models::common::{Distance, Duration, Location as CoreLocation, Profile};
use vrp_core::models::problem::{TransportCost, TravelTime};
use vrp_core::models::solution::Route;
use vrp_core::prelude::Float;

pub fn create_job_place(location: (f64, f64), tag: Option<String>) -> JobPlace {
    JobPlace { times: None, location: location.to_loc(), duration: 1., tag }
}

pub fn create_task(location: (f64, f64), tag: Option<String>) -> JobTask {
    JobTask { places: vec![create_job_place(location, tag)], demand: Some(vec![1]), order: None }
}

pub fn create_job(id: &str) -> Job {
    Job {
        id: id.to_string(),
        pickups: None,
        deliveries: None,
        replacements: None,
        services: None,
        skills: None,
        value: None,
        group: None,
        compatibility: None,
    }
}

pub fn create_delivery_job(id: &str, location: (f64, f64)) -> Job {
    Job { deliveries: Some(vec![create_task(location, None)]), ..create_job(id) }
}

pub fn create_delivery_job_with_order(id: &str, location: (f64, f64), order: i32) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![create_job_place(location, None)],
            demand: Some(vec![1]),
            order: Some(order),
        }]),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_group(id: &str, location: (f64, f64), group: &str) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![create_job_place(location, None)],
            demand: Some(vec![1]),
            order: None,
        }]),
        group: Some(group.to_string()),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_compatibility(id: &str, location: (f64, f64), compatibility: &str) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![create_job_place(location, None)],
            demand: Some(vec![1]),
            order: None,
        }]),
        compatibility: Some(compatibility.to_string()),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_skills(id: &str, location: (f64, f64), skills: JobSkills) -> Job {
    Job { skills: Some(skills), ..create_delivery_job(id, location) }
}

pub fn create_delivery_job_with_demand(id: &str, location: (f64, f64), demand: Vec<i32>) -> Job {
    Job { deliveries: Some(vec![JobTask { demand: Some(demand), ..create_task(location, None) }]), ..create_job(id) }
}

pub fn create_delivery_job_with_duration(id: &str, location: (f64, f64), duration: Duration) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { duration, ..create_job_place(location, None) }],
            demand: Some(vec![1]),
            order: None,
        }]),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_times(
    id: &str,
    location: (f64, f64),
    times: Vec<(i32, i32)>,
    duration: Duration,
) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { duration, times: convert_times(&times), ..create_job_place(location, None) }],
            demand: Some(vec![1]),
            order: None,
        }]),
        ..create_job(id)
    }
}

pub fn create_delivery_job_with_value(id: &str, location: (f64, f64), value: Float) -> Job {
    Job { deliveries: Some(vec![create_task(location, None)]), value: Some(value), ..create_job(id) }
}

pub fn create_pickup_job(id: &str, location: (f64, f64)) -> Job {
    Job { pickups: Some(vec![create_task(location, None)]), ..create_job(id) }
}

pub fn create_pickup_job_with_demand(id: &str, location: (f64, f64), demand: Vec<i32>) -> Job {
    Job { pickups: Some(vec![JobTask { demand: Some(demand), ..create_task(location, None) }]), ..create_job(id) }
}

pub fn create_replacement_job(id: &str, location: (f64, f64)) -> Job {
    Job { replacements: Some(vec![create_task(location, None)]), ..create_job(id) }
}

pub fn create_service_job(id: &str, location: (f64, f64)) -> Job {
    Job { services: Some(vec![JobTask { demand: None, ..create_task(location, None) }]), ..create_job(id) }
}

pub fn create_pickup_delivery_job(id: &str, pickup_location: (f64, f64), delivery_location: (f64, f64)) -> Job {
    Job {
        pickups: Some(vec![create_task(pickup_location, Some("p1".to_string()))]),
        deliveries: Some(vec![create_task(delivery_location, Some("d1".to_string()))]),
        ..create_job(id)
    }
}

pub fn create_pickup_delivery_job_with_params(
    id: &str,
    demand: Vec<i32>,
    pickup: ((f64, f64), Duration, Vec<(i32, i32)>),
    delivery: ((f64, f64), Duration, Vec<(i32, i32)>),
) -> Job {
    Job {
        pickups: Some(vec![JobTask {
            places: vec![JobPlace {
                duration: pickup.1,
                times: convert_times(&pickup.2),
                ..create_job_place(pickup.0, Some("p1".to_string()))
            }],
            demand: Some(demand.clone()),
            order: None,
        }]),
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace {
                duration: delivery.1,
                times: convert_times(&delivery.2),
                ..create_job_place(delivery.0, Some("d1".to_string()))
            }],
            demand: Some(demand),
            order: None,
        }]),

        ..create_job(id)
    }
}

pub fn create_delivery_job_with_index(id: &str, index: usize) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { times: None, location: Location::Reference { index }, duration: 1., tag: None }],
            demand: Some(vec![1]),
            order: None,
        }]),
        ..create_job(id)
    }
}

pub fn create_multi_job(
    id: &str,
    pickups: Vec<((f64, f64), Duration, Vec<i32>)>,
    deliveries: Vec<((f64, f64), Duration, Vec<i32>)>,
) -> Job {
    let create_tasks = |tasks: Vec<((f64, f64), Duration, Vec<i32>)>, prefix: &str| {
        let tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(i, (location, duration, demand))| JobTask {
                places: vec![JobPlace {
                    duration,
                    ..create_job_place((location.0, location.1), Some(format!("{}{}", prefix, i + 1)))
                }],
                demand: Some(demand),
                order: None,
            })
            .collect::<Vec<_>>();

        if tasks.is_empty() { None } else { Some(tasks) }
    };

    Job { pickups: create_tasks(pickups, "p"), deliveries: create_tasks(deliveries, "d"), ..create_job(id) }
}

pub fn create_default_reload() -> VehicleReload {
    VehicleReload { times: None, location: (0., 0.).to_loc(), duration: 2.0, tag: None, resource_id: None }
}

pub fn create_default_vehicle_shift() -> VehicleShift {
    create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))
}

pub fn create_default_open_vehicle_shift() -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
        end: None,
        breaks: None,
        reloads: None,
        recharges: None,
    }
}

pub fn create_default_vehicle_shift_with_locations(start: (f64, f64), end: (f64, f64)) -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(0.), latest: None, location: (start.0, start.1).to_loc() },
        end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (end.0, end.1).to_loc() }),
        breaks: None,
        reloads: None,
        recharges: None,
    }
}

pub fn create_default_vehicle_costs() -> VehicleCosts {
    VehicleCosts { fixed: Some(10.), distance: 1., time: 1. }
}

pub fn create_default_vehicle_profile() -> VehicleProfile {
    VehicleProfile { matrix: "car".to_string(), scale: None }
}

pub fn create_vehicle_profile_with_name(name: &str) -> VehicleProfile {
    VehicleProfile { matrix: name.to_string(), scale: None }
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
        vehicle_ids: vec![format!("{id}_1")],
        profile: create_default_vehicle_profile(),
        costs: create_default_vehicle_costs(),
        shifts: vec![create_default_vehicle_shift()],
        capacity,
        skills: None,
        limits: None,
    }
}

pub fn create_default_fleet() -> Fleet {
    Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_matrix_profiles(), resources: None }
}

pub fn create_default_matrix_profiles() -> Vec<MatrixProfile> {
    vec![MatrixProfile { name: "car".to_string(), speed: None }]
}

pub fn create_min_jobs_cost_objective() -> Option<Vec<Objective>> {
    Some(vec![MinimizeUnassigned { breaks: None }, MinimizeCost])
}

pub fn create_empty_plan() -> Plan {
    Plan { jobs: vec![], relations: None, clustering: None }
}

pub fn create_empty_problem() -> Problem {
    Problem {
        plan: create_empty_plan(),
        fleet: Fleet { vehicles: vec![], profiles: vec![], resources: None },
        objectives: None,
    }
}

pub fn create_matrix(data: Vec<i64>) -> Matrix {
    let size = (data.len() as Float).sqrt() as i32;

    assert_eq!((size * size) as usize, data.len());

    Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: data.clone(),
        distances: data,
        error_codes: None,
    }
}

pub fn create_matrix_from_problem(problem: &Problem) -> Matrix {
    let unique = CoordIndex::new(problem).unique();

    let data: Vec<i64> = unique
        .iter()
        .cloned()
        .flat_map(|a| {
            let (a_lat, a_lng) = a.to_lat_lng();
            unique.iter().map(move |b| {
                let (b_lat, b_lng) = b.to_lat_lng();
                ((a_lat - b_lat).powf(2.) + (a_lng - b_lng).powf(2.)).sqrt().round() as i64
            })
        })
        .collect();

    create_matrix(data)
}

pub fn to_strings(data: Vec<&str>) -> Vec<String> {
    data.iter().map(|item| item.to_string()).collect()
}

pub fn all_of_skills(skills: Vec<String>) -> JobSkills {
    JobSkills { all_of: Some(skills), one_of: None, none_of: None }
}

fn convert_times(times: &[(i32, i32)]) -> Option<Vec<Vec<String>>> {
    if times.is_empty() {
        None
    } else {
        Some(times.iter().map(|tw| vec![format_time(tw.0 as Float), format_time(tw.1 as Float)]).collect())
    }
}

#[derive(Default)]
pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration_approx(&self, _: &Profile, from: CoreLocation, to: CoreLocation) -> Duration {
        fake_routing(from, to)
    }

    fn distance_approx(&self, _: &Profile, from: CoreLocation, to: CoreLocation) -> Distance {
        fake_routing(from, to)
    }

    fn duration(&self, _: &Route, from: CoreLocation, to: CoreLocation, _: TravelTime) -> Duration {
        fake_routing(from, to)
    }

    fn distance(&self, _: &Route, from: CoreLocation, to: CoreLocation, _: TravelTime) -> Distance {
        fake_routing(from, to)
    }

    fn size(&self) -> usize {
        1
    }
}

fn fake_routing(from: CoreLocation, to: CoreLocation) -> Float {
    (if to > from { to - from } else { from - to }) as Float
}
