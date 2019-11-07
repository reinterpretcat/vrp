use crate::helpers::solve_with_cheapest;
use crate::json::problem::*;
use crate::json::solution::writer::create_solution;
use crate::json::solution::*;
use std::sync::Arc;

#[test]
fn can_create_solution() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![JobVariant::Single(Job {
                id: "delivery_job".to_string(),
                places: JobPlaces {
                    pickup: Option::None,
                    delivery: Some(JobPlace {
                        times: Some(vec![
                            vec!["1970-01-01T00:00:00Z".to_string(), "1970-01-01T00:01:40Z".to_string()],
                            vec!["1970-01-01T00:01:50Z".to_string(), "1970-01-01T00:02:00Z".to_string()],
                        ]),
                        location: vec![52.48325, 13.4436],
                        duration: 100.0,
                        tag: Some("my_delivery".to_string()),
                    }),
                },
                demand: vec![1],
                skills: Some(vec!["unique".to_string()]),
            })],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: VehicleCosts { fixed: Some(1.), distance: 1., time: 1. },
                places: VehiclePlaces {
                    start: VehiclePlace { time: "1970-01-01T00:00:00Z".to_string(), location: vec![52.4862, 13.45148] },
                    end: Some(VehiclePlace {
                        time: "1970-01-01T00:01:40Z".to_string(),
                        location: vec![52.4862, 13.45148],
                    }),
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
    let solution = solve_with_cheapest(problem.clone());

    let solution = create_solution(problem.as_ref(), &solution);

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
                    Stop {
                        location: vec![0., 0.],
                        time: Schedule {
                            arrival: "1970-01-01T00:00:00Z".to_string(),
                            departure: "1970-01-01T00:00:00Z".to_string()
                        },
                        load: vec![2],
                        activities: vec![Activity {
                            job_id: "departure".to_string(),
                            activity_type: "departure".to_string(),
                            location: None,
                            time: None,
                            job_tag: None
                        }]
                    },
                    Stop {
                        location: vec![10., 0.],
                        time: Schedule {
                            arrival: "1970-01-01T00:00:10Z".to_string(),
                            departure: "1970-01-01T00:00:10Z".to_string()
                        },
                        load: vec![1],
                        activities: vec![Activity {
                            job_id: "job2".to_string(),
                            activity_type: "delivery".to_string(),
                            location: None,
                            time: None,
                            job_tag: None
                        }]
                    },
                    Stop {
                        location: vec![5., 0.],
                        time: Schedule {
                            arrival: "1970-01-01T00:00:16Z".to_string(),
                            departure: "1970-01-01T00:00:17Z".to_string()
                        },
                        load: vec![0],
                        activities: vec![Activity {
                            job_id: "job1".to_string(),
                            activity_type: "delivery".to_string(),
                            location: None,
                            time: None,
                            job_tag: None
                        }]
                    },
                    Stop {
                        location: vec![0., 0.],
                        time: Schedule {
                            arrival: "1970-01-01T00:00:22Z".to_string(),
                            departure: "1970-01-01T00:00:22Z".to_string()
                        },
                        load: vec![0],
                        activities: vec![Activity {
                            job_id: "arrival".to_string(),
                            activity_type: "arrival".to_string(),
                            location: None,
                            time: None,
                            job_tag: None
                        }]
                    }
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
