use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_one_pickup_delivery_and_two_deliveries_with_one_vehicle() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_pickup_delivery_job("job2", vec![2., 0.], vec![3., 0.]),
                create_delivery_job("job3", vec![4., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet { types: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 30.,
                distance: 8,
                duration: 12,
                times: Timing { driving: 8, serving: 4, waiting: 0, break_time: 0 },
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
                        "job2",
                        "pickup",
                        (2., 0.),
                        3,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (4., 0.),
                        2,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
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
                    cost: 30.,
                    distance: 8,
                    duration: 12,
                    times: Timing { driving: 8, serving: 4, waiting: 0, break_time: 0 },
                }
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
