use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_two_breaks() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (99., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0),
                        latest: Some(format_time(0)),
                        location: (0., 0.).to_loc(),
                    },
                    breaks: Some(vec![
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5), format_time(10)]),
                            places: vec![VehicleOptionalBreakPlace {
                                duration: 2,
                                location: Some((6., 0.).to_loc()),
                                tag: None,
                            }],
                            policy: None,
                        },
                        VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(100), format_time(120)]),
                            places: vec![VehicleOptionalBreakPlace { duration: 2, location: None, tag: None }],
                            policy: None,
                        },
                    ]),
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
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(5, 6)
                            .load(vec![1])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(7, 9)
                            .load(vec![1])
                            .distance(6)
                            .build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((99., 0.))
                            .schedule_stamp(102, 105)
                            .load(vec![0])
                            .distance(99)
                            .activity(
                                ActivityBuilder::delivery()
                                    .job_id("job2")
                                    .coordinate((99., 0.))
                                    .time_stamp(102, 103)
                                    .build()
                            )
                            .activity(ActivityBuilder::break_type().coordinate((99., 0.)).time_stamp(103, 105).build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(204, 204)
                            .load(vec![0])
                            .distance(198)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(198).serving(2).break_time(4).build())
                    .build()
            )
            .build()
    );
}
