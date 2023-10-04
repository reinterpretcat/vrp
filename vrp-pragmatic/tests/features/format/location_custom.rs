use crate::format::problem::*;
use crate::format::{CustomLocationType, Location};
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_unknown_location() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_order("job1", (5., 0.), 1),
                create_delivery_job_with_order("job2", (10., 0.), 2),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0.),
                        latest: None,
                        location: Location::Custom { r#type: CustomLocationType::Unknown },
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
        travel_times: vec![0, 5, 5, 0],
        distances: vec![0, 5, 5, 0],
        error_codes: None,
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default().custom_unknown().schedule_stamp(0., 0.).load(vec![2]).build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(0., 1.)
                            .load(vec![1])
                            .distance(0)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(6., 7.)
                            .load(vec![0])
                            .distance(5)
                            .build_single("job2", "delivery"),
                    ])
                    .statistic(StatisticBuilder::default().driving(5).serving(2).build())
                    .build()
            )
            .build()
    );
}
