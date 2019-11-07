use crate::helpers::{create_delivery_job, create_stop_with_activity, solve_with_heuristic};
use crate::json::problem::*;
use crate::json::solution::*;
use std::sync::Arc;

#[test]
fn can_create_solution() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: VehicleCosts { fixed: Some(10.), distance: 1., time: 1. },
                places: VehiclePlaces {
                    start: VehiclePlace { time: "1970-01-01T00:00:00Z".to_string(), location: vec![0., 0.] },
                    end: Some(VehiclePlace { time: "1970-01-01T00:01:40Z".to_string(), location: vec![0., 0.] }),
                    max_tours: None,
                },
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: None,
                vehicle_break: None,
            }],
        },
    };
    let matrix = Matrix {
        num_origins: 3,
        num_destinations: 3,
        travel_times: vec![0, 5, 5, 5, 0, 10, 5, 10, 0],
        distances: vec![0, 5, 5, 5, 0, 10, 5, 10, 0],
        error_codes: Option::None,
    };
    let problem = Arc::new((problem, vec![matrix]).read_here().unwrap());
    let solution = solve_with_heuristic(problem.clone());

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 52.,
                distance: 20,
                duration: 22,
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
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
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z")
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z")
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z")
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z")
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 20,
                    duration: 22,
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
