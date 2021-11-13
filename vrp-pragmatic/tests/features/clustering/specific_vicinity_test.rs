use super::*;

#[test]
fn can_handle_parking_with_no_clusters_and_job_time_windows() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![52.424, 13.215], vec![(50400, 57600)], 1.),
                create_delivery_job_with_times("job2", vec![52.512, 13.384], vec![(32400, 46800)], 1.),
            ],
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    moving_duration: 30.,
                    moving_distance: 16.,
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

#[test]
fn can_handle_waiting_time_with_parking() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", vec![52.424, 13.2148], vec![(32400, 39600)], 1.),
                create_delivery_job_with_times("job2", vec![52.507, 13.506], vec![(50400, 57600)], 1.),
                create_delivery_job_with_times("job3", vec![52.498, 13.499], vec![(50400, 57600)], 1.),
            ],
            relations: None,
            clustering: Some(Clustering::Vicinity {
                profile: VehicleProfile { matrix: "car".to_string(), scale: None },
                threshold: VicinityThresholdPolicy {
                    moving_duration: 1143.,
                    moving_distance: 128.,
                    min_shared_time: None,
                    smallest_time_window: None,
                    max_jobs_per_cluster: None,
                },
                visiting: VicinityVisitPolicy::Continue,
                serving: VicinityServingPolicy::Original { parking: 300.0 },
                filtering: None,
            }),
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: "1970-01-01T09:00:00Z".to_string(),
                        latest: None,
                        location: Location::Coordinate { lat: 52.505, lng: 13.218 },
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: "1970-01-01T18:00:00Z".to_string(),
                        location: Location::Coordinate { lat: 52.505, lng: 13.218 },
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
