#[cfg(test)]
#[path = "../../../tests/unit/extensions/generate/fleet_test.rs"]
mod fleet_test;

use super::*;
use vrp_pragmatic::format::problem::{Fleet, VehicleCosts, VehicleLimits, VehicleShift, VehicleType};

/// Generates fleet of vehicles.
pub(crate) fn generate_fleet(problem_proto: &Problem, vehicle_types_size: usize) -> Fleet {
    let rnd = DefaultRandom::default();

    let profiles = problem_proto.fleet.profiles.clone();
    let shifts = get_vehicle_shifts(problem_proto);
    let costs = get_vehicle_costs(problem_proto);
    let capacities = get_vehicle_capacities(problem_proto);
    let skills = get_vehicle_skills(problem_proto);
    let limits = get_vehicle_limits(problem_proto);
    let vehicles_sizes = get_vehicles_sizes(problem_proto);

    let vehicles = (1..=vehicle_types_size)
        .map(|type_idx| {
            let type_id = format!("type{}", type_idx);
            let vehicles = *get_random_item(vehicles_sizes.as_slice(), &rnd).expect("cannot find any capacity");
            VehicleType {
                type_id,
                vehicle_ids: (1..=vehicles).map(|vehicle_idx| format!("type{}_{}", type_idx, vehicle_idx)).collect(),
                profile: get_random_item(profiles.as_slice(), &rnd).expect("cannot find any profile").name.clone(),
                costs: get_random_item(costs.as_slice(), &rnd).expect("cannot find any costs").clone(),
                shifts: get_random_item(shifts.as_slice(), &rnd).expect("cannot find any shifts").clone(),
                capacity: get_random_item(capacities.as_slice(), &rnd).expect("cannot find any capacity").clone(),
                skills: get_random_item(skills.as_slice(), &rnd).expect("cannot find any skills").clone(),
                limits: get_random_item(limits.as_slice(), &rnd).expect("cannot find any limits").clone(),
            }
        })
        .collect();

    Fleet { vehicles, profiles }
}

fn get_from_vehicle<F, T>(problem_proto: &Problem, func: F) -> Vec<T>
where
    F: Fn(&VehicleType) -> T,
{
    problem_proto.fleet.vehicles.iter().map(|vehicle| func(vehicle)).collect()
}

fn get_vehicle_costs(problem_proto: &Problem) -> Vec<VehicleCosts> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.costs.clone())
}

fn get_vehicle_shifts(problem_proto: &Problem) -> Vec<Vec<VehicleShift>> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.shifts.clone())
}

fn get_vehicle_capacities(problem_proto: &Problem) -> Vec<Vec<i32>> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.capacity.clone())
}

fn get_vehicle_skills(problem_proto: &Problem) -> Vec<Option<Vec<String>>> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.skills.clone())
}

fn get_vehicle_limits(problem_proto: &Problem) -> Vec<Option<VehicleLimits>> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.limits.clone())
}

fn get_vehicles_sizes(problem_proto: &Problem) -> Vec<usize> {
    get_from_vehicle(problem_proto, |vehicle| vehicle.vehicle_ids.len())
}
