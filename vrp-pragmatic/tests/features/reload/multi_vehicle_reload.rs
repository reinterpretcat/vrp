use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_one_vehicle_with_reload_instead_of_two() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (2., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100), location: (0., 0.).to_loc() }),
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2,
                        ..create_default_reload()
                    }]),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_vehicle_agnostic(
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
                            .coordinate((0., 0.))
                            .schedule_stamp(3, 5)
                            .load(vec![1])
                            .distance(2)
                            .build_single("reload", "reload"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7, 8)
                            .load(vec![0])
                            .distance(4)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(10, 10)
                            .load(vec![0])
                            .distance(6)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(6).serving(4).build())
                    .build(),
            )
            .build(),
    );
}
