use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_vehicle_with_pickups_and_deliveries() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("d1", vec![1., 0.]),
                create_delivery_job("d2", vec![3., 0.]),
                create_delivery_job("d3", vec![3., 0.]),
                create_pickup_job("p1", vec![2., 0.]),
                create_pickup_job("p2", vec![4., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0), location: vec![0., 0.] },
                    end: Some(VehiclePlace { time: format_time(100).to_string(), location: vec![0., 0.] }),
                    breaks: None,
                    max_tours: Some(2),
                    load_time: Some(2),
                }],
                capacity: vec![1],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 40.,
                distance: 12,
                duration: 18,
                times: Timing { driving: 12, serving: 6, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    ),
                    create_stop_with_activity(
                        "d1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    create_stop_with_activity(
                        "p1",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (0., 0.),
                        1,
                        ("1970-01-01T00:00:06Z", "1970-01-01T00:00:08Z"),
                    ),
                    create_stop_with_activity(
                        "d2",
                        "delivery",
                        (3., 0.),
                        0,
                        ("1970-01-01T00:00:11Z", "1970-01-01T00:00:12Z"),
                    ),
                    create_stop_with_activity(
                        "p2",
                        "pickup",
                        (4., 0.),
                        1,
                        ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:18Z"),
                    ),
                ],
                statistic: Statistic {
                    cost: 40.,
                    distance: 12,
                    duration: 18,
                    times: Timing { driving: 12, serving: 6, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![
                UnassignedJob {
                    job_id: "d3".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: 3,
                        description: "does not fit into any vehicle due to capacity".to_string()
                    }],
                }
            ],
            extras: Extras { performance: vec![] },
        }
    );
}
