use crate::format::problem::*;
use crate::helpers::*;
use crate::{format_time, parse_time};
use vrp_core::utils::compare_floats_refs;

#[test]
fn can_use_multiple_times_from_vehicle_and_job() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(0, 100)], 1.),
                create_delivery_job_with_times("job2", (10., 0.), vec![(100, 200)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![
                    VehicleShift {
                        start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                        end: Some(ShiftEnd { earliest: None, latest: format_time(99.), location: (0., 0.).to_loc() }),
                        ..create_default_vehicle_shift()
                    },
                    VehicleShift {
                        start: ShiftStart { earliest: format_time(100.), latest: None, location: (0., 0.).to_loc() },
                        end: Some(ShiftEnd { earliest: None, latest: format_time(200.), location: (0., 0.).to_loc() }),
                        ..create_default_vehicle_shift()
                    },
                ],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_vehicle_agnostic(
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
                            .coordinate((10., 0.))
                            .schedule_stamp(10., 11.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(21., 21.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(1).build())
                    .build(),
            )
            .tour(
                TourBuilder::default()
                    .shift_index(1)
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(100., 100.)
                            .load(vec![1])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(110., 111.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(121., 121.)
                            .load(vec![0])
                            .distance(20)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(20).serving(1).build())
                    .build(),
            )
            .build(),
    );
}

#[test]
fn can_prefer_first_days_with_minimize_arrival_time_objective() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (1., 0.))],
            ..create_empty_plan()
        },
        objectives: Some(vec![
            vec![Objective::MinimizeUnassignedJobs { breaks: None }],
            vec![Objective::MinimizeArrivalTime],
            vec![Objective::MinimizeCost],
        ]),
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: [0., 100., 200., 300., 400.]
                    .iter()
                    .map(|earliest| VehicleShift {
                        start: ShiftStart {
                            earliest: format_time(*earliest),
                            latest: None,
                            location: (0., 0.).to_loc(),
                        },
                        end: None,
                        ..create_default_vehicle_shift()
                    })
                    .collect(),
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    let mut departures = solution
        .tours
        .iter()
        .filter_map(|tour| tour.stops.first())
        .map(|stop| parse_time(&stop.schedule().departure))
        .collect::<Vec<_>>();
    departures.sort_by(compare_floats_refs);
    assert_eq!(departures, vec![0., 100.]);
}
