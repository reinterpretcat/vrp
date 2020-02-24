use crate::format_time;
use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;
use crate::json::Location;
use crate::checker::solve_and_check;

#[test]
fn can_serve_multi_job_and_delivery_with_reload() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple1", vec![1., 0.]),
                create_delivery_job("simple2", vec![3., 0.]),
                create_delivery_job("simple3", vec![7., 0.]),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((9., 0.), 1., vec![2])],
                ),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(100.).to_string(), location: vec![10., 0.].to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: vec![0., 0.].to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
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
                cost: 46.,
                distance: 14,
                duration: 22,
                times: Timing { driving: 14, serving: 8, waiting: 0, break_time: 0 },
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
                        "simple1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    ),
                    create_stop_with_activity(
                        "reload",
                        "reload",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:05Z")
                    ),
                    create_stop_with_activity(
                        "simple2",
                        "delivery",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:08Z", "1970-01-01T00:00:09Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (2., 0.),
                        2,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        "1"
                    ),
                    create_stop_with_activity(
                        "simple3",
                        "delivery",
                        (7., 0.),
                        1,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "pickup",
                        (8., 0.),
                        2,
                        ("1970-01-01T00:00:18Z", "1970-01-01T00:00:19Z"),
                        "2"
                    ),
                    create_stop_with_activity_with_tag(
                        "multi",
                        "delivery",
                        (9., 0.),
                        0,
                        ("1970-01-01T00:00:20Z", "1970-01-01T00:00:21Z"),
                        "1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z"),
                    )
                ],
                statistic: Statistic {
                    cost: 46.,
                    distance: 14,
                    duration: 22,
                    times: Timing { driving: 14, serving: 8, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}

#[test]
fn can_properly_handle_load_without_capacity_violation() {
    let problem = Problem {
        id: "generated_problem_with_reloads".to_string(),
        plan: Plan {
            jobs: vec![
                JobVariant::Single(Job {
                    id: "a6b5a049-248e-43c8-9916-5075dce4d172".to_string(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: None,
                            location: Location { lat: 52.0, lng: 0.0 },
                            duration: 10.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: None,
                            location: Location { lat: 1.0, lng: 0.0 },
                            duration: 12.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "79e70361-ec7c-42ca-9112-cf8578374be6".to_string(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: None,
                            location: Location { lat: 67.0, lng: 0.0 },
                            duration: 16.0,
                            tag: None,
                        }),
                        delivery: None,
                    },
                    demand: vec![2],
                    skills: None,
                }),
            ],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "55094085-ee1e-4ffd-8395-ebbc71eb8535".to_string(),
                profile: "car".to_string(),
                costs: VehicleCosts { fixed: Some(20.0), distance: 0.002, time: 0.003 },
                shifts: vec![VehicleShift {
                    start: VehiclePlace {
                        time: "2020-07-04T09:00:00Z".to_string(),
                        location: Location { lat: 0.0, lng: 0.0 },
                    },
                    end: Some(VehiclePlace {
                        time: "2020-07-04T18:00:00Z".to_string(),
                        location: Location { lat: 0.0, lng: 0.0 },
                    }),
                    breaks: None,
                    reloads: Some(vec![
                        JobPlace {
                            times: None,
                            location: Location { lat: 0.0, lng: 0.0 },
                            duration: 2620.0,
                            tag: None,
                        },
                        JobPlace {
                            times: None,
                            location: Location { lat: 0.0, lng: 0.0 },
                            duration: 2874.0,
                            tag: None,
                        },
                    ]),
                }],
                capacity: vec![2],
                amount: 1,
                skills: None,
                limits: None,
            }],
            profiles: vec![Profile { name: "car".to_string(), profile_type: "car".to_string() }],
        },
        config: None,
    };

    let result = solve_and_check(problem);

    assert_eq!(result, Ok(()));
}
