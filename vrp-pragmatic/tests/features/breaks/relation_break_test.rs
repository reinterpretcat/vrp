use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;
use vrp_core::prelude::Float;

fn get_permissive_break_time() -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeWindow(vec![format_time(0.), format_time(1000.)])
}

fn get_challenging_break_time() -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeWindow(vec![format_time(10.), format_time(15.)])
}

fn get_solution(
    relation_type: RelationType,
    job_duration: Float,
    break_location: Option<Location>,
    break_time: VehicleOptionalBreakTime,
    jobs: Vec<String>,
    perform_check: bool,
) -> Solution {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", (1., 0.), job_duration),
                create_delivery_job_with_duration("job2", (2., 0.), job_duration),
            ],
            relations: Some(vec![Relation {
                type_field: relation_type,
                jobs,
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak::Optional {
                        time: break_time,
                        places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: break_location, tag: None }],
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

    if perform_check {
        solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 100)
    } else {
        solve_with_metaheuristic_and_iterations_without_check(problem, Some(vec![matrix]), 100)
    }
}

parameterized_test! {can_use_break_between_two_jobs_in_relation, relation_type, {
    can_use_break_between_two_jobs_in_relation_impl(relation_type, to_strings(vec!["job1", "break", "job2"]));
}}

can_use_break_between_two_jobs_in_relation! {
    case_01: RelationType::Sequence,
    case_02: RelationType::Strict,
}

fn can_use_break_between_two_jobs_in_relation_impl(relation_type: RelationType, jobs: Vec<String>) {
    let solution = get_solution(relation_type, 1., Some((3., 0.).to_loc()), get_permissive_break_time(), jobs, true);

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
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![1])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(4., 6.)
                            .load(vec![1])
                            .distance(3)
                            .build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![0])
                            .distance(4)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(10., 10.)
                            .load(vec![0])
                            .distance(6)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(6).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

parameterized_test! {can_use_break_last_in_relation, relation_type, {
    can_use_break_last_in_relation_impl(relation_type, to_strings(vec!["job1", "job2", "break"]));
}}

can_use_break_last_in_relation! {
    case_01: RelationType::Sequence,
    case_02: RelationType::Strict,
}

fn can_use_break_last_in_relation_impl(relation_type: RelationType, jobs: Vec<String>) {
    let solution = get_solution(relation_type, 1., Some((3., 0.).to_loc()), get_permissive_break_time(), jobs, true);

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
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![1])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3., 4.)
                            .load(vec![0])
                            .distance(2)
                            .build_single("job2", "delivery"),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(5., 7.)
                            .load(vec![0])
                            .distance(3)
                            .build_single("break", "break"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(10., 10.)
                            .load(vec![0])
                            .distance(6)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(6).serving(2).break_time(2).build())
                    .build()
            )
            .build()
    );
}

#[test]
fn can_stick_to_relation_ignoring_constraint() {
    let relation_type = RelationType::Strict;
    let jobs = to_strings(vec!["departure", "job1", "job2", "break"]);
    let expected = vec![
        to_strings(vec!["departure"]),
        to_strings(vec!["job1"]),
        to_strings(vec!["job2", "break"]),
        to_strings(vec!["arrival"]),
    ];

    let solution = get_solution(relation_type, 10., None, get_challenging_break_time(), jobs, false);

    assert_eq!(solution.tours.len(), 1);
    assert!(solution.unassigned.is_none());
    assert_eq!(get_ids_from_tour(&solution.tours[0]), expected);
}
