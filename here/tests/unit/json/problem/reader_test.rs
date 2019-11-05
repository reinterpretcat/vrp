use crate::helpers::get_test_resource;
use crate::json::problem::*;
use crate::json::HereProblem;

#[test]
fn can_read_minimal_problem() {
    let problem = get_test_resource("../data/small/minimal.problem.json").unwrap();
    let matrix = get_test_resource("../data/small/minimal.matrix.json").unwrap();

    let problem = (problem, vec![matrix]).read_here().unwrap();

    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert_eq!(problem.jobs.all().collect::<Vec<_>>().len(), 2);
    assert!(problem.locks.is_empty());

    let tw = problem
        .fleet
        .vehicles
        .first()
        .as_ref()
        .unwrap()
        .details
        .first()
        .as_ref()
        .unwrap()
        .time
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(tw.start, 1562230800.);
    assert_eq!(tw.end, 1562263200.);
}

#[test]
fn can_read_complex_problem() {
    let problem = Problem {
        id: "problem_id".to_string(),
        plan: Plan {
            jobs: vec![
                JobVariant::Single(Job {
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
                }),
                JobVariant::Single(Job {
                    id: "shipment_job".to_string(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:10Z".to_string(),
                                "1970-01-01T00:00:30Z".to_string(),
                            ]]),
                            location: vec![52.48300, 13.4420],
                            duration: 110.0,
                            tag: None,
                        }),
                        delivery: Some(JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:50Z".to_string(),
                                "1970-01-01T00:01:00Z".to_string(),
                            ]]),
                            location: vec![52.48325, 13.4436],
                            duration: 120.0,
                            tag: None,
                        }),
                    },
                    demand: vec![2],
                    skills: None,
                }),
                JobVariant::Single(Job {
                    id: "pickup_job".to_string(),
                    places: JobPlaces {
                        pickup: Some(JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:10Z".to_string(),
                                "1970-01-01T00:01:10Z".to_string(),
                            ]]),
                            location: vec![52.48321, 13.4438],
                            duration: 90.0,
                            tag: None,
                        }),
                        delivery: Option::None,
                    },
                    demand: vec![3],
                    skills: Some(vec!["unique2".to_string()]),
                }),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: VehicleCosts { fixed: Some(100.), distance: 1., time: 2. },
                places: VehiclePlaces {
                    start: VehiclePlace { time: "1970-01-01T00:00:00Z".to_string(), location: vec![52.4862, 13.45148] },
                    end: Some(VehiclePlace {
                        time: "1970-01-01T00:01:40Z".to_string(),
                        location: vec![52.4862, 13.45148],
                    }),
                    max_tours: Option::None,
                },
                capacity: vec![10],
                amount: 2,
                skills: Some(vec!["unique1".to_string(), "unique2".to_string()]),
                limits: Some(VehicleLimits { max_distance: Some(123.1), shift_time: Some(100.) }),
                vehicle_break: Some(VehicleBreak {
                    times: vec![
                        vec!["1970-01-01T00:00:10Z".to_string(), "1970-01-01T00:01:20Z".to_string()],
                        vec!["1970-01-01T00:01:00Z".to_string(), "1970-01-01T00:03:00Z".to_string()],
                    ],
                    duration: 100.0,
                    location: Some(vec![52.48315, 13.4330]),
                }),
            }],
        },
    };
    let matrix = Matrix {
        num_origins: 5,
        num_destinations: 5,
        travel_times: vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        distances: vec![2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
        error_codes: Option::None,
    };

    let problem = (problem, vec![matrix]).read_here().unwrap();

    assert_eq!(problem.jobs.all().collect::<Vec<_>>().len(), 3 + 2);
    // TODO
}
