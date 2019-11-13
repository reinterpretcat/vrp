use crate::helpers::get_test_resource;
use crate::json::problem::*;
use core::construction::constraints::{Demand, DemandDimension};
use core::models::common::{Dimensions, IdDimension, TimeWindow};
use core::models::problem::{Jobs, Multi, Place, Single};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::sync::Arc;

fn get_job(index: usize, jobs: &Jobs) -> Arc<core::models::problem::Job> {
    jobs.all().collect::<Vec<_>>().get(index).unwrap().clone()
}

fn get_single_job(index: usize, jobs: &Jobs) -> Arc<Single> {
    match get_job(index, jobs).as_ref() {
        core::models::problem::Job::Single(job) => job.clone(),
        _ => panic!("Wrong type"),
    }
}

fn get_multi_job(index: usize, jobs: &Jobs) -> Arc<Multi> {
    match get_job(index, jobs).as_ref() {
        core::models::problem::Job::Multi(job) => job.clone(),
        _ => panic!("Wrong type"),
    }
}

fn get_single_place(single: &Single) -> &Place {
    single.places.first().unwrap()
}

fn assert_time_window(tw: &TimeWindow, expected: &(f64, f64)) {
    assert_eq!(tw.start, expected.0);
    assert_eq!(tw.end, expected.1);
}

fn assert_time_windows(tws: &Vec<TimeWindow>, expected: Vec<(f64, f64)>) {
    assert_eq!(tws.len(), expected.len());
    (0..tws.len()).for_each(|index| {
        assert_time_window(tws.get(index).unwrap(), expected.get(index).unwrap());
    });
}

fn assert_demand(demand: &Demand<i32>, expected: &Demand<i32>) {
    assert_eq!(demand.pickup.0, expected.pickup.0);
    assert_eq!(demand.pickup.1, expected.pickup.1);
    assert_eq!(demand.delivery.0, expected.delivery.0);
    assert_eq!(demand.delivery.1, expected.delivery.1);
}

fn assert_skills(dimens: &Dimensions, expected: Option<Vec<String>>) {
    let skills = dimens.get("skills").and_then(|any| any.downcast_ref::<HashSet<String>>());
    if let Some(expected) = expected {
        let expected = HashSet::from_iter(expected.iter().cloned());
        assert_eq!(skills.unwrap().clone(), expected);
    } else {
        assert!(skills.is_none());
    }
}

#[test]
fn can_read_minimal_problem() {
    let problem = get_test_resource("../data/small/minimal.problem.json").unwrap();
    let matrix = get_test_resource("../data/small/minimal.matrix.json").unwrap();

    let problem = (problem, vec![matrix]).read_here().unwrap();

    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert_eq!(problem.jobs.all().collect::<Vec<_>>().len(), 2);
    assert!(problem.locks.is_empty());

    assert_time_window(
        problem.fleet.vehicles.first().as_ref().unwrap().details.first().as_ref().unwrap().time.as_ref().unwrap(),
        &(1562230800., 1562263200.),
    );
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

    // delivery
    let job = get_single_job(0, problem.jobs.as_ref());
    let place = get_single_place(job.as_ref());
    assert_eq!(job.dimens.get_id().unwrap(), "delivery_job");
    assert_eq!(place.duration, 100.);
    assert_eq!(place.location.unwrap(), 0);
    assert_demand(job.dimens.get_demand().unwrap(), &Demand::<i32> { pickup: (0, 0), delivery: (1, 0) });
    assert_time_windows(&place.times, vec![(0., 100.), (110., 120.)]);
    assert_skills(&job.dimens, Some(vec!["unique".to_string()]));

    // shipment
    let job = get_multi_job(1, problem.jobs.as_ref());
    assert_eq!(job.dimens.get_id().unwrap(), "shipment_job");
    assert_skills(&job.dimens, None);

    let pickup = job.jobs.first().unwrap().clone();
    let place = get_single_place(pickup.as_ref());
    assert_eq!(place.duration, 110.);
    assert_eq!(place.location.unwrap(), 1);
    assert_demand(pickup.dimens.get_demand().unwrap(), &Demand::<i32> { pickup: (0, 2), delivery: (0, 0) });
    assert_time_windows(&place.times, vec![(10., 30.)]);

    let delivery = job.jobs.last().unwrap().clone();
    let place = get_single_place(delivery.as_ref());
    assert_eq!(place.duration, 120.);
    assert_eq!(place.location.unwrap(), 0);
    assert_demand(delivery.dimens.get_demand().unwrap(), &Demand::<i32> { pickup: (0, 0), delivery: (0, 2) });
    assert_time_windows(&place.times, vec![(50., 60.)]);

    // pickup
    let job = get_single_job(2, problem.jobs.as_ref());
    let place = get_single_place(job.as_ref());
    assert_eq!(job.dimens.get_id().unwrap(), "pickup_job");
    assert_eq!(place.duration, 90.);
    assert_eq!(place.location.unwrap(), 2);
    assert_demand(job.dimens.get_demand().unwrap(), &Demand::<i32> { pickup: (3, 0), delivery: (0, 0) });
    assert_time_windows(&place.times, vec![(10., 70.)]);
    assert_skills(&job.dimens, Some(vec!["unique2".to_string()]));

    // fleet
    assert_eq!(problem.fleet.profiles.len(), 1);
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 2);

    (1..3).for_each(|index| {
        let vehicle = problem.fleet.vehicles.get(index - 1).unwrap();
        assert_eq!(*vehicle.dimens.get_id().unwrap(), format!("my_vehicle_{}", index));
        assert_eq!(vehicle.profile, 0);
        assert_eq!(vehicle.costs.fixed, 100.0);
        assert_eq!(vehicle.costs.per_distance, 1.0);
        assert_eq!(vehicle.costs.per_driving_time, 2.0);
        assert_eq!(vehicle.costs.per_waiting_time, 2.0);
        assert_eq!(vehicle.costs.per_service_time, 2.0);

        assert_eq!(vehicle.details.len(), 1);
        let detail = vehicle.details.first().unwrap();
        assert_eq!(detail.start.unwrap(), 3);
        assert_eq!(detail.end.unwrap(), 3);
        assert_time_window(detail.time.as_ref().unwrap(), &(0., 100.));
        assert_skills(&vehicle.dimens, Some(vec!["unique1".to_string(), "unique2".to_string()]));
    });
}
