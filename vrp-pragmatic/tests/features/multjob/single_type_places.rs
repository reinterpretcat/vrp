use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_only_deliveries_as_static_demand() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job("job1", vec![], vec![((8., 0.), 2, vec![1]), ((2., 0.), 1, vec![1])])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
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
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2, 3)
                            .load(vec![1])
                            .distance(2)
                            .build_single_tag("job1", "delivery", "d2"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(9, 11)
                            .load(vec![0])
                            .distance(8)
                            .build_single_tag("job1", "delivery", "d1"),
                    ])
                    .statistic(StatisticBuilder::default().driving(8).serving(3).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_use_only_pickups_as_static_demand() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job("job1", vec![((8., 0.), 2, vec![1]), ((2., 0.), 1, vec![1])], vec![])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000), location: (10., 0.).to_loc() }),
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
                            .load(vec![0])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(2, 3)
                            .load(vec![1])
                            .distance(2)
                            .build_single_tag("job1", "pickup", "p2"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(9, 11)
                            .load(vec![2])
                            .distance(8)
                            .build_single_tag("job1", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13, 13)
                            .load(vec![0])
                            .distance(10)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(10).serving(3).build())
                    .build()
            )
            .build()
    );
}
