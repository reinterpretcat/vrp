use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

fn create_vehicle_type_with_max_duration_limit(max_duration: f64) -> VehicleType {
    VehicleType {
        limits: Some(VehicleLimits { max_distance: None, max_duration: Some(max_duration), tour_size: None }),
        ..create_default_vehicle_type()
    }
}

#[test]
fn can_limit_one_job_by_max_duration() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (100., 0.))], ..create_empty_plan() },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(99.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![1, 100, 100, 1],
        distances: vec![1, 1, 1, 1],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .unassigned(Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "MAX_DURATION_CONSTRAINT".to_string(),
                    description: "cannot be assigned due to max duration constraint of vehicle".to_string(),
                    details: None
                }]
            }]))
            .build()
    );
}

#[test]
fn can_skip_job_from_multiple_because_of_max_duration() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", (1., 0.), 10.),
                create_delivery_job_with_duration("job2", (2., 0.), 10.),
                create_delivery_job_with_duration("job3", (3., 0.), 10.),
                create_delivery_job_with_duration("job4", (4., 0.), 10.),
                create_delivery_job_with_duration("job5", (5., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(40.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![3])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(3., 13.)
                            .load(vec![2])
                            .distance(3)
                            .build_single("job3", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(14., 24.)
                            .load(vec![1])
                            .distance(4)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(25., 35.)
                            .load(vec![0])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(36., 36.)
                            .load(vec![0])
                            .distance(6)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(6).serving(30).build())
                    .build()
            )
            .unassigned(Some(vec![
                UnassignedJob {
                    job_id: "job4".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: "MAX_DURATION_CONSTRAINT".to_string(),
                        description: "cannot be assigned due to max duration constraint of vehicle".to_string(),
                        details: Some(vec![UnassignedJobDetail {
                            vehicle_id: "my_vehicle_1".to_string(),
                            shift_index: 0
                        }]),
                    }]
                },
                UnassignedJob {
                    job_id: "job5".to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: "MAX_DURATION_CONSTRAINT".to_string(),
                        description: "cannot be assigned due to max duration constraint of vehicle".to_string(),
                        details: Some(vec![UnassignedJobDetail {
                            vehicle_id: "my_vehicle_1".to_string(),
                            shift_index: 0
                        }]),
                    }]
                }
            ]))
            .build()
    );
}

#[test]
// NOTE: this is a specific use case of departure time optimization
#[ignore]
fn can_serve_job_when_it_starts_late() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(100, 200)], 10.)],
            ..create_empty_plan()
        },
        fleet: Fleet { vehicles: vec![create_vehicle_type_with_max_duration_limit(50.)], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert!(!solution.tours.is_empty());
}
