use crate::format_time;
use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

fn get_solution(relation_type: RelationType, jobs: Vec<String>) -> Solution {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![2., 0.])],
            relations: Some(vec![Relation {
                type_field: relation_type,
                jobs,
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    breaks: Some(vec![VehicleBreak {
                        times: VehicleBreakTime::TimeWindows(vec![vec![format_time(0.), format_time(1000.)]]),
                        duration: 2.0,
                        locations: Some(vec![vec![3., 0.].to_loc()]),
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    solve_with_metaheuristic(problem, vec![matrix])
}

parameterized_test! {can_use_break_between_two_jobs_in_relation, relation_type, {
    can_use_break_between_two_jobs_in_relation_impl(relation_type, to_strings(vec!["job1", "break", "job2"]));
}}

can_use_break_between_two_jobs_in_relation! {
    case_01: RelationType::Sequence,
    case_02: RelationType::Strict,
}

fn can_use_break_between_two_jobs_in_relation_impl(relation_type: RelationType, jobs: Vec<String>) {
    let solution = get_solution(relation_type, jobs);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 2, waiting: 0, break_time: 2 },
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
                    times: Timing { driving: 6, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            unassigned: vec![],
            extras: None,
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
    let solution = get_solution(relation_type, jobs);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 26.,
                distance: 6,
                duration: 10,
                times: Timing { driving: 6, serving: 2, waiting: 0, break_time: 2 },
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
                    times: Timing { driving: 6, serving: 2, waiting: 0, break_time: 2 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}
