use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_two_dimensions() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_demand("job1", vec![1., 0.], vec![0, 1]),
                create_delivery_job_with_demand("job2", vec![2., 0.], vec![1, 0]),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![1, 1],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 16.,
                distance: 2,
                duration: 4,
                times: Timing { driving: 2, serving: 2, waiting: 0, break_time: 0 },
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
                        vec![1, 0],
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity_md(
                        "job2",
                        "delivery",
                        (2., 0.),
                        vec![0, 0],
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2
                    )
                ],
                statistic: Statistic {
                    cost: 16.,
                    distance: 2,
                    duration: 4,
                    times: Timing { driving: 2, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}

#[test]
fn can_unassign_due_to_dimension_mismatch() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job_with_demand("job1", vec![1., 0.], vec![0, 1])], relations: None },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![1],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 0.,
                distance: 0,
                duration: 0,
                times: Timing { driving: 0, serving: 0, waiting: 0, break_time: 0 },
            },
            tours: vec![],
            unassigned: vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: 3,
                    description: "does not fit into any vehicle due to capacity".to_string()
                }]
            }],
            extras: None,
        }
    );
}
