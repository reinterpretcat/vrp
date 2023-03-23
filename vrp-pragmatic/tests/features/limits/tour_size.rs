use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_skip_job_from_multiple_because_of_tour_size() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                limits: Some(VehicleLimits { max_distance: None, shift_time: None, tour_size: Some(2) }),
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
                cost: 16.,
                distance: 2,
                duration: 4,
                times: Timing { driving: 2, serving: 2, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2
                    )
                ],
                statistic: Statistic {
                    cost: 16.,
                    distance: 2,
                    duration: 4,
                    times: Timing { driving: 2, serving: 2, ..Timing::default() },
                },
            }],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job3".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "TOUR_SIZE_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to tour size constraint of vehicle".to_string(),
                    details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
