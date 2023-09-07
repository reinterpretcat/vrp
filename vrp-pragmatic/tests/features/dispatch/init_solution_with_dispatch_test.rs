use crate::core::prelude::Solver;
use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::assert_vehicle_agnostic;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::rosomaxa::evolution::TelemetryMode;
use vrp_core::solver::create_default_config_builder;
use vrp_core::utils::Environment;

#[test]
fn can_use_init_solution_with_dispatch() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job_with_times("job2", (1., 0.), vec![(7, 100)], 1.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1".to_string(), "v2".to_string()],
                shifts: vec![VehicleShift {
                    dispatch: Some(vec![VehicleDispatch {
                        location: (0., 0.).to_loc(),
                        limits: vec![
                            VehicleDispatchLimit { max: 1, start: format_time(2.), end: format_time(4.) },
                            VehicleDispatchLimit { max: 1, start: format_time(4.), end: format_time(6.) },
                        ],
                        tag: None,
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
    let init_solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .vehicle_id("v1")
                .stops(vec![
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(0., 4.)
                        .load(vec![1])
                        .distance(0)
                        .activity(
                            ActivityBuilder::default()
                                .activity_type("departure")
                                .job_id("departure")
                                .time_stamp(0., 2.)
                                .build(),
                        )
                        .activity(
                            ActivityBuilder::default()
                                .activity_type("dispatch")
                                .job_id("dispatch")
                                .time_stamp(2., 4.)
                                .build(),
                        )
                        .build(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(5., 6.)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(7., 7.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(3).build())
                .build(),
        )
        .tour(
            TourBuilder::default()
                .vehicle_id("v2")
                .stops(vec![
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(0., 6.)
                        .load(vec![1])
                        .distance(0)
                        .activity(
                            ActivityBuilder::default()
                                .activity_type("departure")
                                .job_id("departure")
                                .time_stamp(0., 4.)
                                .build(),
                        )
                        .activity(
                            ActivityBuilder::default()
                                .activity_type("dispatch")
                                .job_id("dispatch")
                                .time_stamp(4., 6.)
                                .build(),
                        )
                        .build(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(7., 8.)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(9., 9.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(3).build())
                .build(),
        )
        .build();
    let environment = Arc::new(Environment::default());
    let matrix = create_matrix_from_problem(&problem);
    let core_problem = Arc::new((problem, vec![matrix]).read_pragmatic().unwrap());
    let core_solution = to_core_solution(&init_solution, core_problem.clone(), environment.random.clone()).unwrap();

    let core_solution = create_default_config_builder(core_problem.clone(), environment.clone(), TelemetryMode::None)
        .with_max_generations(Some(100))
        .with_init_solutions(
            vec![InsertionContext::new_from_solution(core_problem.clone(), (core_solution, None), environment)],
            None,
        )
        .build()
        .map(|config| Solver::new(core_problem.clone(), config))
        .unwrap_or_else(|err| panic!("cannot build solver: {err}"))
        .solve()
        .unwrap_or_else(|err| panic!("cannot solve the problem: {err}"));
    let result_solution = create_solution(&core_problem, &core_solution, &Default::default());

    assert_vehicle_agnostic(result_solution, init_solution);
}
