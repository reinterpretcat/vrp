use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_service_job() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_service_job("job2", (2., 0.)),
                create_pickup_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000), location: (4., 0.).to_loc() }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
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
                            .schedule_stamp(0, 0)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1, 2)
                            .load(vec![0])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3, 4)
                            .load(vec![0])
                            .distance(2)
                            .build_single("job2", "service"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(5, 6)
                            .load(vec![1])
                            .distance(3)
                            .build_single("job3", "pickup"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(7, 7)
                            .load(vec![0])
                            .distance(4)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(4).serving(3).build())
                    .build()
            )
            .build()
    );
}
