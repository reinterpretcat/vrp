use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_limit_by_max_distance() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (100., 0.))], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits { max_distance: Some(99.), shift_time: None, tour_size: None, areas: None }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1, 1, 1, 1],
        distances: vec![1, 100, 100, 1],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic::default(),
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "MAX_DISTANCE_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to max distance constraint of vehicle".to_string(),
                    details: None
                }]
            }]),
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_handle_empty_route() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (5., 0.))], ..create_empty_plan() },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100.), location: (10., 0.).to_loc() }),
                    ..create_default_open_vehicle_shift()
                }],
                limits: Some(VehicleLimits { max_distance: Some(9.), shift_time: None, tour_size: None, areas: None }),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic::default(),
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "MAX_DISTANCE_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to max distance constraint of vehicle".to_string(),
                    details: None,
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
