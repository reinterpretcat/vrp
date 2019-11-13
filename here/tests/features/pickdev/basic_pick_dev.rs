use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_one_pickup_delivery_job_with_one_vehicle() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan { jobs: vec![create_pickup_delivery_job("job1", vec![1., 0.], vec![2., 0.])], relations: None },
        fleet: Fleet { types: vec![create_default_vehicle("my_vehicle")] },
    };
    let matrix = Matrix {
        num_origins: 3,
        num_destinations: 3,
        travel_times: vec![0, 1, 1, 1, 0, 2, 1, 2, 0],
        distances: vec![0, 1, 1, 1, 0, 2, 1, 2, 0],
        error_codes: Option::None,
    };

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 20.,
                distance: 4,
                duration: 6,
                times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 0 },
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
                    create_stop_with_activity(
                        "job1",
                        "pickup",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:06Z", "1970-01-01T00:00:06Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 20.,
                    distance: 4,
                    duration: 6,
                    times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
