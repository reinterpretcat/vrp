use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_handle_limited_capacity() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_multi_job(
                    "multi_1",
                    vec![((1., 0.), 1., vec![1]), ((2., 0.), 1., vec![1])],
                    vec![((10., 0.), 1., vec![2])],
                ),
                create_multi_job(
                    "multi_2",
                    vec![((3., 0.), 1., vec![1]), ((4., 0.), 1., vec![1])],
                    vec![((11., 0.), 1., vec![2])],
                ),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 88.,
                distance: 36,
                duration: 42,
                times: Timing { driving: 36, serving: 6, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_1",
                        "pickup",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        "1",
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_1",
                        "pickup",
                        (2., 0.),
                        2,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        "2",
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_1",
                        "delivery",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:13Z"),
                        "1",
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_2",
                        "pickup",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:21Z"),
                        "1",
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_2",
                        "pickup",
                        (4., 0.),
                        2,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:23Z"),
                        "2",
                    ),
                    create_stop_with_activity_with_tag(
                        "multi_2",
                        "delivery",
                        (11., 0.),
                        0,
                        ("1970-01-01T00:00:30Z", "1970-01-01T00:00:31Z"),
                        "1",
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:42Z", "1970-01-01T00:00:42Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 88.,
                    distance: 36,
                    duration: 42,
                    times: Timing { driving: 36, serving: 6, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
