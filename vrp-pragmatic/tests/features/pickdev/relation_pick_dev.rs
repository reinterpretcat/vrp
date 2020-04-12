use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_two_pickup_delivery_jobs_and_relation_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_pickup_delivery_job("job1", vec![20., 0.], vec![15., 0.]),
                create_pickup_delivery_job("job2", vec![5., 0.], vec![20., 0.]),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Sequence,
                jobs: to_strings(vec!["job1", "job2", "job1", "job2"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
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
                cost: 114.,
                distance: 50,
                duration: 54,
                times: Timing { driving: 50, serving: 4, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "pickup",
                        (20., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job2",
                        "pickup",
                        (5., 0.),
                        2,
                        ("1970-01-01T00:00:26Z", "1970-01-01T00:00:27Z"),
                        25
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (15., 0.),
                        1,
                        ("1970-01-01T00:00:37Z", "1970-01-01T00:00:38Z"),
                        35
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (20., 0.),
                        0,
                        ("1970-01-01T00:00:43Z", "1970-01-01T00:00:44Z"),
                        40
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:54Z", "1970-01-01T00:00:54Z"),
                        50
                    )
                ],
                statistic: Statistic {
                    cost: 114.,
                    distance: 50,
                    duration: 54,
                    times: Timing { driving: 50, serving: 4, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
