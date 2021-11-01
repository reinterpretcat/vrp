use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_multiple_times() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![10., 0.], vec![(0, 10)], 0.),
                create_delivery_job_with_times("job2", vec![20., 0.], vec![(10, 20)], 0.),
                create_delivery_job_with_times("job3", vec![30., 0.], vec![(5, 8), (100, 120)], 0.),
                create_delivery_job_with_times("job4", vec![40., 0.], vec![(30, 40)], 0.),
                create_delivery_job_with_times("job5", vec![50., 0.], vec![(40, 50)], 0.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("my_vehicle")],
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
                cost: 240.,
                distance: 100,
                duration: 130,
                times: Timing { driving: 100, serving: 0, waiting: 30, break_time: 0 },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (10., 0.),
                        4,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (20., 0.),
                        3,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:20Z"),
                        20
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (40., 0.),
                        2,
                        ("1970-01-01T00:00:40Z", "1970-01-01T00:00:40Z"),
                        40
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (50., 0.),
                        1,
                        ("1970-01-01T00:00:50Z", "1970-01-01T00:00:50Z"),
                        50
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (30., 0.),
                        0,
                        ("1970-01-01T00:01:10Z", "1970-01-01T00:01:40Z"),
                        70
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:02:10Z", "1970-01-01T00:02:10Z"),
                        100
                    )
                ],
                statistic: Statistic {
                    cost: 240.,
                    distance: 100,
                    duration: 130,
                    times: Timing { driving: 100, serving: 0, waiting: 30, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
