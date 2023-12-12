use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

parameterized_test! {can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload, generations, {
    can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload_impl(generations);
}}

can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload! {
    case01: 1,
    case02: 200,
}

fn can_serve_multi_job_and_delivery_in_one_tour_avoiding_reload_impl(generations: usize) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple", (1., 0.)),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((6., 0.), 1., vec![2])],
                ),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100.), location: (0., 0.).to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2.0,
                        ..create_default_reload()
                    }]),
                    recharges: None,
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), generations);

    assert_eq!(
        solution,
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![0])
                            .distance(1)
                            .build_single("simple", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3., 4.)
                            .load(vec![1])
                            .distance(2)
                            .build_single_tag("multi", "pickup", "p1"),
                        StopBuilder::default()
                            .coordinate((8., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![2])
                            .distance(8)
                            .build_single_tag("multi", "pickup", "p2"),
                        StopBuilder::default()
                            .coordinate((6., 0.))
                            .schedule_stamp(13., 14.)
                            .load(vec![0])
                            .distance(10)
                            .build_single_tag("multi", "delivery", "d1"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(20., 20.)
                            .load(vec![0])
                            .distance(16)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(16).serving(4).build())
                    .build()
            )
            .build()
    );
}
