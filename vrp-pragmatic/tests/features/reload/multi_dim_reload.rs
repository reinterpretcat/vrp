use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_multi_dim_capacity() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![1, 1]),
                create_delivery_job_with_demand("job2", (2., 0.), vec![1, 1]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100.), location: (0., 0.).to_loc() }),
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2.0,
                        ..create_default_reload()
                    }]),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![1, 1],
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
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 4, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity_md(
                        "departure",
                        "departure",
                        (0., 0.),
                        vec![1, 1],
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity_md(
                        "job1",
                        "delivery",
                        (1., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity_md(
                        "reload",
                        "reload",
                        (0., 0.),
                        vec![1, 1],
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:05Z"),
                        2
                    ),
                    create_stop_with_activity_md(
                        "job2",
                        "delivery",
                        (2., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        4
                    ),
                    create_stop_with_activity_md(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        6
                    ),
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 6,
                    duration: 10,
                    times: Timing { driving: 6, serving: 4, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_use_multi_dim_capacity_with_different_values() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", (1., 0.), vec![1, 1]),
                create_delivery_job_with_demand("job2", (2., 0.), vec![1, 1]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    reloads: Some(vec![create_default_reload()]),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![2, 1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
}
