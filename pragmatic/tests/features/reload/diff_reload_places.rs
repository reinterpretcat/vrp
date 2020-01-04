use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_reloads_with_different_locations() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![10., 0.]),
                create_delivery_job("job2", vec![11., 0.]),
                create_delivery_job("job3", vec![20., 0.]),
                create_delivery_job("job4", vec![21., 0.]),
                create_delivery_job("job5", vec![30., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(1000), location: vec![32., 0.].to_loc() }),
                    breaks: None,
                    reloads: Some(vec![
                        VehicleReload {
                            times: None,
                            location: vec![12., 0.].to_loc(),
                            duration: 2.0,
                            tag: Some("close".to_string()),
                        },
                        VehicleReload {
                            times: None,
                            location: vec![33., 0.].to_loc(),
                            duration: 2.0,
                            tag: Some("far".to_string()),
                        },
                    ]),
                }],
                capacity: vec![2],
                amount: 1,
                skills: None,
                limits: None,
            }],
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
                cost: 95.,
                distance: 38,
                duration: 47,
                times: Timing { driving: 38, serving: 9, waiting: 0, break_time: 0 },
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
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (11., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:13Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "reload",
                        "reload",
                        (12., 0.),
                        2,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:16Z"),
                        "close"
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (20., 0.),
                        1,
                        ("1970-01-01T00:00:24Z", "1970-01-01T00:00:25Z"),
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (21., 0.),
                        0,
                        ("1970-01-01T00:00:26Z", "1970-01-01T00:00:27Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "reload",
                        "reload",
                        (33., 0.),
                        1,
                        ("1970-01-01T00:00:39Z", "1970-01-01T00:00:41Z"),
                        "far"
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (30., 0.),
                        0,
                        ("1970-01-01T00:00:44Z", "1970-01-01T00:00:45Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (32., 0.),
                        0,
                        ("1970-01-01T00:00:47Z", "1970-01-01T00:00:47Z"),
                    ),
                ],
                statistic: Statistic {
                    cost: 95.,
                    distance: 38,
                    duration: 47,
                    times: Timing { driving: 38, serving: 9, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
