use super::*;

#[test]
fn can_handle_parking_with_no_clusters_and_job_time_windows() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (52.424, 13.215), vec![(50400, 57600)], 1.),
                create_delivery_job_with_times("job2", (52.512, 13.384), vec![(32400, 46800)], 1.),
            ],
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    duration: 30.,
                    distance: 16.,
                    min_shared_time: None,
                    smallest_time_window: None,
                    max_jobs_per_cluster: None,
                },
                visiting: VicinityVisitPolicy::Continue,
                serving: VicinityServingPolicy::Original { parking: 300. },
                filtering: None,
            }),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T09:00:00Z".to_string(),
                        latest: None,
                        location: Location::Coordinate { lat: 52.497, lng: 13.547 },
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: "1970-01-01T18:00:00Z".to_string(),
                        location: Location::Coordinate { lat: 52.497, lng: 13.547 },
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle("vehicle1")
            }],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        ..create_empty_problem()
    };

    let matrices = create_approx_matrices(&problem);
    solve_with_metaheuristic_and_iterations(problem, Some(matrices), 1);
}

parameterized_test! {can_handle_waiting_time_with_parking, (jobs, threshold, vehicle_location), {
    let vehicle_location = Location::Coordinate { lat: vehicle_location.0, lng: vehicle_location.1 };
    can_handle_waiting_time_with_parking_impl(jobs, threshold, vehicle_location);
}}

can_handle_waiting_time_with_parking! {
    case_01: (vec![
            ("job1", (52.424, 13.2148), vec![(32400, 39600)]),
            ("job2", (52.507, 13.506), vec![(50400, 57600)]),
            ("job3", (52.498, 13.499), vec![(50400, 57600)]),
        ],
        (1143., 128.), (52.505, 13.218),
    ),
    case_02: (vec![
            ("job1", (52.559, 13.228), vec![(50400, 64800)]),
            ("job2", (52.575, 13.395), vec![(32400, 39600)]),
            ("job3", (52.575, 13.395), vec![(32400, 39600)]),
        ],
        (210., 930.), (52.577, 13.530),
    ),
}

fn can_handle_waiting_time_with_parking_impl(
    jobs: Vec<(&str, (f64, f64), Vec<(i32, i32)>)>,
    threshold: (f64, f64),
    vehicle_location: Location,
) {
    let problem = Problem {
        plan: Plan {
            jobs: jobs
                .into_iter()
                .map(|(id, coordinates, times)| create_delivery_job_with_times(id, coordinates, times, 1.))
                .collect(),
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    duration: threshold.0,
                    distance: threshold.1,
                    min_shared_time: None,
                    smallest_time_window: None,
                    max_jobs_per_cluster: None,
                },
                visiting: VicinityVisitPolicy::Continue,
                serving: VicinityServingPolicy::Original { parking: 300.0 },
                filtering: None,
            }),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T09:00:00Z".to_string(),
                        latest: None,
                        location: vehicle_location.clone(),
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: "1970-01-01T18:00:00Z".to_string(),
                        location: vehicle_location,
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle("vehicle1")
            }],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        objectives: None,
    };

    let matrices = create_approx_matrices(&problem);
    solve_with_metaheuristic_and_iterations(problem, Some(matrices), 1);
}
