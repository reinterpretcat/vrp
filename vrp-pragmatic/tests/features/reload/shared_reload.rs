use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

const CAPACITY_CODE: &str = "CAPACITY_CONSTRAINT";
const RESOURCE_CODE: &str = "RELOAD_RESOURCE_CONSTRAINT";

fn create_test_jobs(amount: usize) -> Vec<Job> {
    (0..amount).map(|idx| create_delivery_job(format!("job{}", idx + 1).as_str(), (1., 0.))).collect()
}

fn get_reasons(solution: &Solution) -> Vec<Vec<String>> {
    solution
        .unassigned
        .iter()
        .flat_map(|unassigned| unassigned.iter())
        .map(|u_job| u_job.reasons.iter().map(|reason| reason.code.clone()).collect::<Vec<_>>())
        .collect()
}

parameterized_test! {can_consume_limited_resource_with_single_vehicle, (vehicle_capacity, resource_capacity, reloads, expected_unassigned), {
    can_consume_limited_resource_with_single_vehicle_impl(vehicle_capacity, resource_capacity, reloads, expected_unassigned);
}}

can_consume_limited_resource_with_single_vehicle! {
    case01: (2, 1, 1, vec![vec![RESOURCE_CODE]]),
    case02: (2, 0, 1, vec![vec![CAPACITY_CODE], vec![CAPACITY_CODE]]),
    case03: (2, 2, 1, vec![]),

    case04: (1, 1, 1, vec![vec![CAPACITY_CODE], vec![CAPACITY_CODE]]),
    case05: (1, 2, 1, vec![vec![CAPACITY_CODE], vec![CAPACITY_CODE]]),

    case06: (1, 2, 2, vec![vec![CAPACITY_CODE]]),
    case07: (1, 2, 3, vec![vec![CAPACITY_CODE]]),
}

fn can_consume_limited_resource_with_single_vehicle_impl(
    vehicle_capacity: i32,
    resource_capacity: i32,
    reloads: usize,
    expected_unassigned: Vec<Vec<&str>>,
) {
    let problem = Problem {
        plan: Plan { jobs: create_test_jobs(4), ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    reloads: Some(
                        (0..reloads)
                            .map(|_| VehicleReload {
                                resource_id: Some("resource_1".to_string()),
                                ..create_default_reload()
                            })
                            .collect(),
                    ),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![vehicle_capacity],
                ..create_default_vehicle_type()
            }],
            resources: Some(vec![VehicleResource::Reload {
                id: "resource_1".to_string(),
                capacity: vec![resource_capacity],
            }]),
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(get_reasons(&solution), expected_unassigned);
}

parameterized_test! {can_consume_limited_resource_with_two_vehicles, (vehicles, jobs_amount, is_open_shift, resources, expected_unassigned), {
    can_consume_limited_resource_with_two_vehicles_impl(vehicles, jobs_amount, is_open_shift, resources, expected_unassigned);
}}

can_consume_limited_resource_with_two_vehicles! {
    case01_two_resources:
        (vec![("res1", 1), ("res2", 1)], 4, false, vec![("res1", 1), ("res2", 1)], vec![]),
    case02_one_resource_not_enough:
        (vec![("res1", 1), ("res1", 1)], 4, false, vec![("res1", 1)], vec![vec![CAPACITY_CODE, CAPACITY_CODE]]),
    case03_one_resource_enough:
        (vec![("res1", 1), ("res2", 1)], 4, false, vec![("res1", 2), ("res2", 2)], vec![]),

    case04_open_shift:
        (vec![("res1", 1), ("res1", 1)], 4, true, vec![("res1", 1)], vec![vec![CAPACITY_CODE, CAPACITY_CODE]]),
    case05_open_shift:
        (vec![("res1", 2)], 4, true, vec![("res1", 1)], vec![vec![RESOURCE_CODE]]),
}

fn can_consume_limited_resource_with_two_vehicles_impl(
    vehicles: Vec<(&str, i32)>,
    jobs_amount: usize,
    is_open_shift: bool,
    resources: Vec<(&str, i32)>,
    expected_unassigned: Vec<Vec<&str>>,
) {
    let problem = Problem {
        plan: Plan { jobs: create_test_jobs(jobs_amount), ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vehicles
                .into_iter()
                .enumerate()
                .map(|(idx, (id, capacity))| VehicleType {
                    type_id: format!("type_{}", idx + 1),
                    vehicle_ids: vec![format!("v{}", idx + 1)],
                    shifts: vec![VehicleShift {
                        end: if is_open_shift { None } else { create_default_vehicle_shift().end },
                        reloads: Some(vec![VehicleReload {
                            resource_id: Some(id.to_string()),
                            ..create_default_reload()
                        }]),
                        ..create_default_vehicle_shift()
                    }],
                    capacity: vec![capacity],
                    ..create_default_vehicle_type()
                })
                .collect(),
            resources: Some(
                resources
                    .into_iter()
                    .map(|(id, capacity)| VehicleResource::Reload { id: id.to_string(), capacity: vec![capacity] })
                    .collect(),
            ),
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(get_reasons(&solution), expected_unassigned);
}
