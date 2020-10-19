use crate::format::problem::*;
use crate::format::{CoordIndex, Location};
use crate::format_time;
use crate::helpers::ToLocation;

pub fn create_job_place(location: Vec<f64>) -> JobPlace {
    JobPlace { times: None, location: location.to_loc(), duration: 1. }
}

pub fn create_task(location: Vec<f64>) -> JobTask {
    JobTask { places: vec![create_job_place(location)], demand: Some(vec![1]), tag: None }
}

pub fn create_job(id: &str) -> Job {
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

pub fn create_delivery_job_with_skills(id: &str, location: Vec<f64>, skills: JobSkills) -> Job {
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
        pickups: Some(vec![JobTask { tag: Some("p1".to_string()), ..create_task(pickup_location.clone()) }]),
        deliveries: Some(vec![JobTask { tag: Some("d1".to_string()), ..create_task(delivery_location.clone()) }]),
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
            tag: Some("p1".to_string()),
        }]),
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace {
                duration: delivery.1,
                times: convert_times(&delivery.2),
                ..create_job_place(delivery.0.clone())
            }],
            demand: Some(demand.clone()),
            tag: Some("d1".to_string()),
        }]),

        ..create_job(id)
    }
}

pub fn create_delivery_job_with_index(id: &str, index: usize) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace { times: None, location: Location::Reference { index }, duration: 1. }],
            demand: Some(vec![1]),
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
    let create_tasks = |tasks: Vec<((f64, f64), f64, Vec<i32>)>, prefix: &str| {
        let tasks = tasks
            .into_iter()
            .enumerate()
            .map(|(i, (location, duration, demand))| JobTask {
                places: vec![JobPlace { duration, ..create_job_place(vec![location.0, location.1]) }],
                demand: Some(demand),
                tag: Some(format!("{}{}", prefix, i + 1)),
            })
            .collect::<Vec<_>>();

        if tasks.is_empty() {
            None
        } else {
            Some(tasks)
        }
    };

    Job { pickups: create_tasks(pickups, "p"), deliveries: create_tasks(deliveries, "d"), ..create_job(id) }
}

pub fn create_default_vehicle_shift() -> VehicleShift {
    create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))
}

pub fn create_default_open_vehicle_shift() -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(0.), latest: None, location: vec![0., 0.].to_loc() },
        end: None,
        dispatch: None,
        breaks: None,
        reloads: None,
    }
}

pub fn create_default_vehicle_shift_with_locations(start: (f64, f64), end: (f64, f64)) -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(0.), latest: None, location: vec![start.0, start.1].to_loc() },
        end: Some(ShiftEnd {
            earliest: None,
            latest: format_time(1000.).to_string(),
            location: vec![end.0, end.1].to_loc(),
        }),
        dispatch: None,
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
    vec![Profile { name: "car".to_string(), profile_type: "car".to_string(), speed: None }]
}

pub fn create_empty_problem() -> Problem {
    Problem {
        plan: Plan { jobs: vec![], relations: None },
        fleet: Fleet { vehicles: vec![], profiles: vec![] },
        objectives: None,
    }
}

pub fn create_matrix(data: Vec<i64>) -> Matrix {
    let size = (data.len() as f64).sqrt() as i32;

    assert_eq!((size * size) as usize, data.len());

    Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
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

fn convert_times(times: &Vec<(i32, i32)>) -> Option<Vec<Vec<String>>> {
    if times.is_empty() {
        None
    } else {
        Some(times.iter().map(|tw| vec![format_time(tw.0 as f64), format_time(tw.1 as f64)]).collect())
    }
}
