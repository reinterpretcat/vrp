use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_break_using_second_place() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (10., 0.)), create_delivery_job("job2", (20., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (30., 0.).to_loc() }),
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(10.), format_time(30.)]),
                        places: vec![
                            VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((1., 0.).to_loc()),
                                tag: Some("first".to_string()),
                            },
                            VehicleOptionalBreakPlace {
                                duration: 2.0,
                                location: Some((11., 0.).to_loc()),
                                tag: Some("second".to_string()),
                            },
                        ],
                        policy: None,
                    }]),
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
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![1])
                            .distance(10)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((11., 0.))
                            .schedule_stamp(12., 14.)
                            .load(vec![1])
                            .distance(11)
                            .build_single_tag("break", "break", "second"),
                        StopBuilder::default()
                            .coordinate((20., 0.))
                            .schedule_stamp(23., 24.)
                            .load(vec![0])
                            .distance(20)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((30., 0.))
                            .schedule_stamp(34., 34.)
                            .load(vec![0])
                            .distance(30)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(30).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}
