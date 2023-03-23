use crate::format::problem::*;
use crate::format::solution::{UnassignedJobDetail, UnassignedJobReason};
use crate::helpers::*;

#[test]
fn can_separate_jobs_based_on_compatibility() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_compatibility("food", (1., 0.), "food"),
                create_delivery_job("job2", (8., 0.)),
                create_delivery_job_with_compatibility("junk", (2., 0.), "junk"),
                create_delivery_job("job4", (9., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                VehicleType {
                    type_id: "type1".to_string(),
                    vehicle_ids: vec!["type1_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (0., 0.))],
                    capacity: vec![2],
                    ..create_default_vehicle_type()
                },
                VehicleType {
                    type_id: "type2".to_string(),
                    vehicle_ids: vec!["type2_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
                    capacity: vec![2],
                    ..create_default_vehicle_type()
                },
            ],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert!(solution.unassigned.is_none());
    let junk_tour = solution.tours.iter().find(|tour| tour.vehicle_id == "type2_1").unwrap();
    let food_tour = solution.tours.iter().find(|tour| tour.vehicle_id == "type1_1").unwrap();
    assert_eq!(get_ids_from_tour(junk_tour).iter().flatten().filter(|id| *id == "junk" || *id == "job4").count(), 2);
    assert_eq!(get_ids_from_tour(food_tour).iter().flatten().filter(|id| *id == "food" || *id == "job2").count(), 2);
}

#[test]
fn can_unassign_job_due_to_compatibility() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_compatibility("food", (1., 0.), "food"),
                create_delivery_job_with_compatibility("junk", (2., 0.), "junk"),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType { capacity: vec![2], ..create_default_vehicle_type() }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.unassigned.as_ref().map_or(0, |u| u.len()), 1);
    let reasons = solution.unassigned.iter().flatten().flat_map(|u| u.reasons.iter().cloned()).collect::<Vec<_>>();
    assert_eq!(
        reasons,
        vec![UnassignedJobReason {
            code: "COMPATIBILITY_CONSTRAINT".to_string(),
            description: "cannot be assigned due to compatibility constraint".to_string(),
            details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }])
        }]
    );
}
