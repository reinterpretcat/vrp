use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_skip_break_when_vehicle_not_used() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![
                VehicleType {
                    id: "my_vehicle".to_string(),
                    profile: "car".to_string(),
                    costs: create_default_vehicle_costs(),
                    places: create_default_vehicle_places_with_locations((100., 0.), (100., 0.)),
                    capacity: vec![10],
                    amount: 1,
                    skills: None,
                    limits: None,
                    vehicle_break: Some(VehicleBreak {
                        times: vec![vec![format_time(5), format_time(8)]],
                        duration: 2.0,
                        location: Some(vec![6., 0.]),
                    }),
                },
                create_default_vehicle("vehicle_without_break"),
            ],
        },
    };
    let matrix = Matrix {
        num_origins: 5,
        num_destinations: 5,
        travel_times: vec![0, 5, 95, 1, 5, 5, 0, 90, 4, 10, 95, 90, 0, 94, 100, 1, 4, 94, 0, 6, 5, 10, 100, 6, 0],
        distances: vec![0, 5, 95, 1, 5, 5, 0, 90, 4, 10, 95, 90, 0, 94, 100, 1, 4, 94, 0, 6, 5, 10, 100, 6, 0],
        error_codes: Option::None,
    };

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

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
                vehicle_id: "vehicle_without_break_1".to_string(),
                type_id: "vehicle_without_break".to_string(),
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
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z"),
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

#[test]
fn can_skip_break_when_jobs_completed() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![JobVariant::Single(Job {
                id: "job1".to_string(),
                places: JobPlaces {
                    pickup: Option::None,
                    delivery: Some(JobPlace { times: None, location: vec![1., 0.], duration: 10., tag: None }),
                },
                demand: vec![1],
                skills: None,
            })],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                places: create_default_vehicle_places(),
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: None,
                vehicle_break: Some(VehicleBreak {
                    times: vec![vec![format_time(5), format_time(8)]],
                    duration: 2.0,
                    location: Some(vec![6., 0.]),
                }),
            }],
        },
    };
    let matrix = Matrix {
        num_origins: 5,
        num_destinations: 5,
        travel_times: vec![0, 1, 5, 1, 0, 6, 5, 6, 0],
        distances: vec![0, 1, 5, 1, 0, 6, 5, 6, 0],
        error_codes: Option::None,
    };

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 24.,
                distance: 2,
                duration: 12,
                times: Timing { driving: 2, serving: 10, waiting: 0, break_time: 0 },
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
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:11Z"),
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
                    cost: 24.,
                    distance: 2,
                    duration: 12,
                    times: Timing { driving: 2, serving: 10, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        }
    );
}
