use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;

fn get_permissive_break_time() -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeWindow(vec![format_time(0.), format_time(1000.)])
}

fn get_challenging_break_time() -> VehicleOptionalBreakTime {
    VehicleOptionalBreakTime::TimeWindow(vec![format_time(10.), format_time(15.)])
}

fn get_solution(
    relation_type: RelationType,
    job_duration: f64,
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
            profiles: create_default_matrix_profiles(),
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
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 2, break_time: 2, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "break",
                        "break",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:04Z", "1970-01-01T00:00:06Z"),
                        3
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        4
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        6
                    )
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 6,
                    duration: 10,
                    times: Timing { driving: 6, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
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
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 2, break_time: 2, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2
                    ),
                    create_stop_with_activity(
                        "break",
                        "break",
                        (3., 0.),
                        0,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:07Z"),
                        3
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:10Z"),
                        6
                    )
                ],
                statistic: Statistic {
                    cost: 26.,
                    distance: 6,
                    duration: 10,
                    times: Timing { driving: 6, serving: 2, break_time: 2, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
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
