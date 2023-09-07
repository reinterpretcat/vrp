use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_assign_break_between_jobs() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (5., 0.)), create_delivery_job("job2", (10., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: create_default_vehicle_costs(),
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(5.), format_time(10.)]),
                        places: vec![VehicleOptionalBreakPlace {
                            duration: 2.0,
                            location: Some((6., 0.).to_loc()),
                            tag: Some("break_tag".to_string()),
                        }],
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
                            .coordinate((5., 0.))
                            .schedule_stamp(5., 6.)
                            .load(vec![1])
                            .distance(5)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(7., 9.)
                            .load(vec![1])
                            .distance(6)
                            .build_single_tag("break", "break", "break_tag"),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(24., 24.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}
