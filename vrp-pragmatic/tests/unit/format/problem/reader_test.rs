use crate::format::problem::*;
use crate::format::{JobTie, VehicleTie};
use crate::helpers::*;
use hashbrown::HashSet;
use std::iter::FromIterator;
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::{Jobs, Multi, Place, Single};

fn get_job(index: usize, jobs: &Jobs) -> vrp_core::models::problem::Job {
    jobs.all().collect::<Vec<_>>().get(index).unwrap().clone()
}

fn get_single_job(index: usize, jobs: &Jobs) -> Arc<Single> {
    get_job(index, jobs).to_single().clone()
}

fn get_multi_job(index: usize, jobs: &Jobs) -> Arc<Multi> {
    get_job(index, jobs).to_multi().clone()
}

fn get_single_place(single: &Single) -> &Place {
    single.places.first().unwrap()
}

fn assert_time_window(tw: &TimeWindow, expected: &(f64, f64)) {
    assert_eq!(tw.start, expected.0);
    assert_eq!(tw.end, expected.1);
}

fn assert_time_spans(tws: &[TimeSpan], expected: Vec<(f64, f64)>) {
    assert_eq!(tws.len(), expected.len());
    (0..tws.len()).for_each(|index| {
        assert_time_window(&tws.get(index).and_then(|tw| tw.as_time_window()).unwrap(), expected.get(index).unwrap());
    });
}

fn assert_demand(demand: &Demand<MultiDimLoad>, expected: &Demand<MultiDimLoad>) {
    assert_eq!(demand.pickup.0.as_vec(), expected.pickup.0.as_vec());
    assert_eq!(demand.pickup.1.as_vec(), expected.pickup.1.as_vec());
    assert_eq!(demand.delivery.0.as_vec(), expected.delivery.0.as_vec());
    assert_eq!(demand.delivery.1.as_vec(), expected.delivery.1.as_vec());
}

fn assert_job_skills(dimens: &Dimensions, expected: Option<Vec<String>>) {
    let skills = dimens.get_job_skills();
    if let Some(expected) = expected {
        let expected = HashSet::from_iter(expected.iter().cloned());
        assert_eq!(skills.unwrap().all_of, Some(expected));
    } else {
        assert!(skills.is_none());
    }
}

fn assert_vehicle_skills(dimens: &Dimensions, expected: Option<Vec<String>>) {
    let skills = dimens.get_vehicle_skills();
    if let Some(expected) = expected {
        let expected = HashSet::from_iter(expected.iter().cloned());
        assert_eq!(skills.unwrap().clone(), expected);
    } else {
        assert!(skills.is_none());
    }
}

#[test]
fn can_read_complex_problem() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                Job {
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            times: Some(vec![
                                vec!["1970-01-01T00:00:00Z".to_string(), "1970-01-01T00:01:40Z".to_string()],
                                vec!["1970-01-01T00:01:50Z".to_string(), "1970-01-01T00:02:00Z".to_string()],
                            ]),
                            location: (52.48325, 13.4436).to_loc(),
                            duration: 100.0,
                            tag: Some("my_delivery".to_string()),
                        }],
                        demand: Some(vec![0, 1]),
                        order: None,
                    }]),
                    skills: Some(all_of_skills(vec!["unique".to_string()])),
                    ..create_job("delivery_job")
                },
                Job {
                    pickups: Some(vec![JobTask {
                        places: vec![JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:10Z".to_string(),
                                "1970-01-01T00:00:30Z".to_string(),
                            ]]),
                            location: (52.48300, 13.4420).to_loc(),
                            duration: 110.0,
                            tag: None,
                        }],
                        demand: Some(vec![2]),
                        order: None,
                    }]),
                    deliveries: Some(vec![JobTask {
                        places: vec![JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:50Z".to_string(),
                                "1970-01-01T00:01:00Z".to_string(),
                            ]]),
                            location: (52.48325, 13.4436).to_loc(),
                            duration: 120.0,
                            tag: None,
                        }],
                        demand: Some(vec![2]),
                        order: None,
                    }]),
                    ..create_job("pickup_delivery_job")
                },
                Job {
                    pickups: Some(vec![JobTask {
                        places: vec![JobPlace {
                            times: Some(vec![vec![
                                "1970-01-01T00:00:10Z".to_string(),
                                "1970-01-01T00:01:10Z".to_string(),
                            ]]),
                            location: (52.48321, 13.4438).to_loc(),
                            duration: 90.0,
                            tag: None,
                        }],
                        demand: Some(vec![3]),
                        order: None,
                    }]),
                    skills: Some(all_of_skills(vec!["unique2".to_string()])),
                    ..create_job("pickup_job")
                },
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                type_id: "my_vehicle".to_string(),
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                profile: create_default_vehicle_profile(),
                costs: VehicleCosts { fixed: Some(100.), distance: 1., time: 2. },
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T00:00:00Z".to_string(),
                        latest: None,
                        location: (52.4862, 13.45148).to_loc(),
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: "1970-01-01T00:01:40Z".to_string(),
                        location: (52.4862, 13.45148).to_loc(),
                    }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![
                            "1970-01-01T00:00:10Z".to_string(),
                            "1970-01-01T00:01:20Z".to_string(),
                        ]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 100.0,
                            location: Some((52.48315, 13.4330).to_loc()),
                            tag: None,
                        }],
                        policy: None,
                    }]),
                    reloads: None,
                    recharges: None,
                }],
                capacity: vec![10, 1],
                skills: Some(vec!["unique1".to_string(), "unique2".to_string()]),
                limits: Some(VehicleLimits { max_distance: Some(123.1), max_duration: Some(100.), tour_size: Some(3) }),
            }],
            ..create_default_fleet()
        },
        objectives: None,
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1; 25],
        distances: vec![2; 25],
        error_codes: None,
    };

    let problem = (problem, vec![matrix]).read_pragmatic().ok().unwrap();

    assert_eq!(problem.jobs.all().count(), 3 + 2);

    // delivery
    let job = get_single_job(0, problem.jobs.as_ref());
    let place = get_single_place(job.as_ref());
    assert_eq!(job.dimens.get_job_id().unwrap(), "delivery_job");
    assert_eq!(place.duration, 100.);
    assert_eq!(place.location.unwrap(), 0);
    assert_demand(
        job.dimens.get_demand().unwrap(),
        &Demand {
            pickup: (MultiDimLoad::default(), MultiDimLoad::default()),
            delivery: (MultiDimLoad::new(vec![0, 1]), MultiDimLoad::default()),
        },
    );
    assert_time_spans(&place.times, vec![(0., 100.), (110., 120.)]);
    assert_job_skills(&job.dimens, Some(vec!["unique".to_string()]));

    // shipment
    let job = get_multi_job(1, problem.jobs.as_ref());
    assert_eq!(job.dimens.get_job_id().unwrap(), "pickup_delivery_job");
    assert_job_skills(&job.dimens, None);

    let pickup = job.jobs.first().unwrap().clone();
    let place = get_single_place(pickup.as_ref());
    assert_eq!(place.duration, 110.);
    assert_eq!(place.location.unwrap(), 1);
    assert_demand(pickup.dimens.get_demand().unwrap(), &single_demand_as_multi((0, 2), (0, 0)));
    assert_time_spans(&place.times, vec![(10., 30.)]);

    let delivery = job.jobs.last().unwrap().clone();
    let place = get_single_place(delivery.as_ref());
    assert_eq!(place.duration, 120.);
    assert_eq!(place.location.unwrap(), 0);
    assert_demand(delivery.dimens.get_demand().unwrap(), &single_demand_as_multi((0, 0), (0, 2)));
    assert_time_spans(&place.times, vec![(50., 60.)]);

    // pickup
    let job = get_single_job(2, problem.jobs.as_ref());
    let place = get_single_place(job.as_ref());
    assert_eq!(job.dimens.get_job_id().unwrap(), "pickup_job");
    assert_eq!(place.duration, 90.);
    assert_eq!(place.location.unwrap(), 2);
    assert_demand(job.dimens.get_demand().unwrap(), &single_demand_as_multi((3, 0), (0, 0)));
    assert_time_spans(&place.times, vec![(10., 70.)]);
    assert_job_skills(&job.dimens, Some(vec!["unique2".to_string()]));

    // fleet
    assert_eq!(problem.fleet.profiles.len(), 1);
    assert_eq!(problem.fleet.drivers.len(), 1);
    assert_eq!(problem.fleet.vehicles.len(), 2);

    (1..3).for_each(|index| {
        let vehicle = problem.fleet.vehicles.get(index - 1).unwrap();
        assert_eq!(*vehicle.dimens.get_vehicle_id().unwrap(), format!("my_vehicle_{index}"));
        assert_eq!(vehicle.profile.index, 0);
        assert_eq!(vehicle.profile.scale, 1.);
        assert_eq!(vehicle.costs.fixed, 100.0);
        assert_eq!(vehicle.costs.per_distance, 1.0);
        assert_eq!(vehicle.costs.per_driving_time, 2.0);
        assert_eq!(vehicle.costs.per_waiting_time, 2.0);
        assert_eq!(vehicle.costs.per_service_time, 2.0);

        assert_eq!(vehicle.details.len(), 1);
        let detail = vehicle.details.first().unwrap();
        assert_eq!(detail.start.as_ref().unwrap().location, 3);
        assert_eq!(detail.end.as_ref().unwrap().location, 3);
        assert_time_window(
            &TimeWindow::new(
                detail.start.as_ref().unwrap().time.earliest.unwrap(),
                detail.end.as_ref().unwrap().time.latest.unwrap(),
            ),
            &(0., 100.),
        );
        assert_vehicle_skills(&vehicle.dimens, Some(vec!["unique1".to_string(), "unique2".to_string()]));
    });
}

#[test]
fn can_deserialize_minimal_problem_and_matrix() {
    let problem = (SIMPLE_PROBLEM.to_string(), vec![SIMPLE_MATRIX.to_string()]).read_pragmatic().ok().unwrap();

    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert_eq!(problem.jobs.all().count(), 2);
    assert!(problem.locks.is_empty());

    let detail = problem.fleet.vehicles.first().unwrap().details.first().unwrap();
    assert_time_window(
        &TimeWindow::new(
            detail.start.as_ref().unwrap().time.earliest.unwrap(),
            detail.end.as_ref().unwrap().time.latest.unwrap(),
        ),
        &(1562230800., 1562263200.),
    );
}

#[test]
fn can_create_approximation_matrices() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (52.52599, 13.45413)),
                create_delivery_job("job2", (52.5165, 13.3808)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![],
            profiles: vec![
                MatrixProfile { name: "car1".to_string(), speed: Some(8.) },
                MatrixProfile { name: "car2".to_string(), speed: Some(10.) },
                MatrixProfile { name: "car3".to_string(), speed: Some(5.) },
                MatrixProfile { name: "car4".to_string(), speed: None },
            ],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let matrices = create_approx_matrices(&problem);
    assert_eq!(matrices.len(), 4);

    for &(profile, duration) in &[("car1", 635), ("car2", 508), ("car3", 1016), ("car4", 508)] {
        let matrix = matrices.iter().find(|m| m.profile.as_ref().unwrap().as_str() == profile).unwrap();

        assert!(matrix.error_codes.is_none());
        assert!(matrix.timestamp.is_none());

        assert_eq!(matrix.distances, &[0, 5078, 5078, 0]);
        assert_eq!(matrix.travel_times, &[0, duration, duration, 0]);
    }
}
