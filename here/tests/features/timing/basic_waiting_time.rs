use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_wait_for_job_start() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![1., 0.], vec![(0, 1)], 0.),
                create_delivery_job_with_times("job2", vec![2., 0.], vec![(10, 20)], 0.),
            ],
            relations: Option::None,
        },
        fleet: Fleet { types: vec![create_default_vehicle("my_vehicle")] },
    };
    let matrix = create_matrix(vec![0, 1, 1, 1, 0, 2, 1, 2, 0]);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 26.,
                distance: 4,
                duration: 12,
                times: Timing { driving: 4, serving: 0, waiting: 8, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:01Z"),
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:10Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 4,
                    duration: 12,
                    times: Timing { driving: 4, serving: 0, waiting: 8, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
