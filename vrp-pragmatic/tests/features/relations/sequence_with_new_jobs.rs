use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_sequence_relation_with_strict_time_windows() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![10., 0.], vec![(150, 170)], 10.),
                create_delivery_job_with_times("job2", vec![20., 0.], vec![(20, 30)], 10.),
                create_delivery_job_with_times("job3", vec![30., 0.], vec![(40, 50)], 10.),
                create_delivery_job_with_times("job4", vec![40., 0.], vec![(60, 150)], 10.),
                create_delivery_job_with_times("job5", vec![50., 0.], vec![(70, 80)], 10.),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Sequence,
                jobs: to_strings(vec!["job5", "job4"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string()],
                capacity: vec![10],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 270.,
                distance: 100,
                duration: 160,
                times: Timing { driving: 100, serving: 50, waiting: 10, ..Timing::default() },
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
                        5,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:10Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (20., 0.),
                        4,
                        ("1970-01-01T00:00:30Z", "1970-01-01T00:00:40Z"),
                        20
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (30., 0.),
                        3,
                        ("1970-01-01T00:00:50Z", "1970-01-01T00:01:00Z"),
                        30
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (50., 0.),
                        2,
                        ("1970-01-01T00:01:20Z", "1970-01-01T00:01:30Z"),
                        50
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (40., 0.),
                        1,
                        ("1970-01-01T00:01:40Z", "1970-01-01T00:01:50Z"),
                        60
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:02:20Z", "1970-01-01T00:02:40Z"),
                        90
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:02:50Z", "1970-01-01T00:02:50Z"),
                        100
                    )
                ],
                statistic: Statistic {
                    cost: 270.,
                    distance: 100,
                    duration: 160,
                    times: Timing { driving: 100, serving: 50, waiting: 10, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
