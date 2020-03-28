use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_any_relation_with_new_job_for_one_vehicle_with_open_end() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Any,
                jobs: to_strings(vec!["job1", "job3"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![3],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 19.,
                distance: 3,
                duration: 6,
                times: Timing { driving: 3, serving: 3, waiting: 0, break_time: 0 },
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
                        3,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        2,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1,
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2,
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (3., 0.),
                        0,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        3,
                    )
                ],
                statistic: Statistic {
                    cost: 19.,
                    distance: 3,
                    duration: 6,
                    times: Timing { driving: 3, serving: 3, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
