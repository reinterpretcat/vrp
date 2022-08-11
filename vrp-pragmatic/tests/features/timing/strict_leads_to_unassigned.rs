use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_have_unassigned_jobs_because_of_strict_times() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 10)], 0.),
                create_delivery_job_with_times("job2", (20., 0.), vec![(10, 20)], 0.),
                create_delivery_job_with_times("job3", (30., 0.), vec![(20, 30)], 0.),
                create_delivery_job_with_times("job4", (40., 0.), vec![(30, 40)], 0.),
                create_delivery_job_with_times("job5", (50., 0.), vec![(0, 10)], 0.),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 170.,
                distance: 80,
                duration: 80,
                times: Timing { driving: 80, serving: 0, ..Timing::default() },
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
                        4,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (10., 0.),
                        3,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (20., 0.),
                        2,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:20Z"),
                        20
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (30., 0.),
                        1,
                        ("1970-01-01T00:00:30Z", "1970-01-01T00:00:30Z"),
                        30
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (40., 0.),
                        0,
                        ("1970-01-01T00:00:40Z", "1970-01-01T00:00:40Z"),
                        40
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:01:20Z", "1970-01-01T00:01:20Z"),
                        80
                    ),
                ],
                statistic: Statistic {
                    cost: 170.,
                    distance: 80,
                    duration: 80,
                    times: Timing { driving: 80, serving: 0, ..Timing::default() },
                },
            }],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job5".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "TIME_WINDOW_CONSTRAINT".to_string(),
                    description: "cannot be visited within time window".to_string(),
                    details: Some(vec![UnassignedJobDetail { vehicle_id: "my_vehicle_1".to_string(), shift_index: 0 }]),
                }]
            }]),
            ..create_empty_solution()
        },
    );
}
