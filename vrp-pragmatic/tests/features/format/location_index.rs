use crate::format::Location;
use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_location_index() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_index("job1", 0), create_delivery_job_with_index("job2", 1)],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: None,
                        location: Location::Reference { index: 2 },
                    },
                    ..create_default_open_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_string()),
        timestamp: None,
        travel_times: vec![0, 3, 3, 1, 0, 3, 3, 2, 0],
        distances: vec![0, 3, 3, 1, 0, 3, 3, 2, 0],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default().reference(2).schedule_stamp(0., 0.).load(vec![2]).build_departure(),
                        StopBuilder::default()
                            .reference(1)
                            .schedule_stamp(2., 3.)
                            .load(vec![1])
                            .distance(2)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .reference(0)
                            .schedule_stamp(4., 5.)
                            .load(vec![0])
                            .distance(3)
                            .build_single("job1", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(3).serving(2).build())
                    .build()
            )
            .build()
    );
}
