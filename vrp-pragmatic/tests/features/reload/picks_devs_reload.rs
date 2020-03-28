use crate::format_time;
use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_vehicle_with_pickups_and_deliveries() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("d1", vec![1., 0.]),
                create_delivery_job("d2", vec![4., 0.]),
                create_delivery_job("d3", vec![10., 0.]),
                create_pickup_job("p1", vec![2., 0.]),
                create_pickup_job("p2", vec![5., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(100.).to_string(), location: vec![6., 0.].to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: vec![3., 0.].to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![1],
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
                cost: 28.,
                distance: 6,
                duration: 12,
                times: Timing { driving: 6, serving: 6, waiting: 0, break_time: 0 },
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
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "d1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "p1",
                        "pickup",
                        (2., 0.),
                        1,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:07Z"),
                        3
                    ),
                    create_stop_with_activity(
                        "d2",
                        "delivery",
                        (4., 0.),
                        0,
                        ("1970-01-01T00:00:08Z", "1970-01-01T00:00:09Z"),
                        4
                    ),
                    create_stop_with_activity(
                        "p2",
                        "pickup",
                        (5., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        5
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (6., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                        6
                    ),
                ],
                statistic: Statistic {
                    cost: 28.,
                    distance: 6,
                    duration: 12,
                    times: Timing { driving: 6, serving: 6, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![UnassignedJob {
                job_id: "d3".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: 3,
                    description: "does not fit into any vehicle due to capacity".to_string()
                }],
            }],
            extras: None,
        }
    );
}
