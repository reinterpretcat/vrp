use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

fn create_test_problem() -> Problem {
    Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![2., 0.])],
            relations: None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_matrix_profiles() },
        ..create_empty_problem()
    }
}

fn create_test_statistic() -> Statistic {
    Statistic {
        cost: 10.,
        distance: 4,
        duration: 6,
        times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 0 },
    }
}

fn create_test_solution(statistic: Statistic, stop_data: &[(f64, i64); 3]) -> Solution {
    let [first, second, third] = stop_data;
    Solution {
        statistic: statistic.clone(),
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
                    0,
                ),
                Stop {
                    location: vec![1., 0.].to_loc(),
                    time: Schedule { arrival: format_time(first.0), departure: "1970-01-01T00:00:02Z".to_string() },
                    distance: first.1,
                    load: vec![1],
                    activities: vec![Activity {
                        job_id: "job1".to_string(),
                        activity_type: "delivery".to_string(),
                        location: None,
                        time: None,
                        job_tag: None,
                        commute: None,
                    }],
                },
                Stop {
                    location: vec![2., 0.].to_loc(),
                    time: Schedule { arrival: format_time(second.0), departure: "1970-01-01T00:00:04Z".to_string() },
                    distance: second.1,
                    load: vec![0],
                    activities: vec![Activity {
                        job_id: "job2".to_string(),
                        activity_type: "delivery".to_string(),
                        location: Some(vec![2., 0.].to_loc()),
                        time: None,
                        job_tag: None,
                        commute: None,
                    }],
                },
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    0,
                    (format_time(third.0).as_str(), "1970-01-01T00:00:06Z"),
                    third.1,
                ),
            ],
            statistic,
        }],
        ..create_empty_solution()
    }
}

fn duration_error_msg(stop_idx: usize, actual: usize, expected: usize) -> String {
    format!("arrival time mismatch for {} stop in the tour: my_vehicle_1, expected: '1970-01-01T00:00:0{}Z', got: '1970-01-01T00:00:0{}Z'",
        stop_idx,
        expected,
        actual,
    )
}

fn distance_error_msg(stop_idx: usize, actual: usize, expected: usize) -> String {
    format!(
        "distance mismatch for {} stop in the tour: my_vehicle_1, expected: '{}', got: '{}'",
        stop_idx, expected, actual,
    )
}

parameterized_test! {can_check_stop, (stop_data, expected_result), {
    can_check_stop_impl(stop_data, expected_result);
}}

can_check_stop! {
    case_01: (&[(1., 1), (3., 2), (6., 4)], Ok(())),

    // NOTE due to rounding issues, we have to compare with tolerance 1
    case_02: (&[(2., 1), (3., 2), (6., 4)], Ok(())),
    case_03: (&[(1., 1), (3., 1), (6., 4)], Ok(())),

    case_04: (&[(3., 1), (3., 2), (6., 4)], Err(vec![duration_error_msg(1, 3, 1)])),
    case_05: (&[(1., 1), (1., 2), (6., 4)], Err(vec![duration_error_msg(2, 1, 3)])),
    case_06: (&[(1., 1), (3., 2), (8., 4)], Err(vec![duration_error_msg(3, 8, 6)])),

    case_07: (&[(1., 3), (3., 2), (6., 4)], Err(vec![distance_error_msg(1, 3, 1)])),
    case_08: (&[(1., 1), (3., 0), (6., 4)], Err(vec![distance_error_msg(2, 0, 2)])),
    case_09: (&[(1., 1), (3., 2), (6., 6)], Err(vec![distance_error_msg(3, 6, 4)])),
}

fn can_check_stop_impl(stop_data: &[(f64, i64); 3], expected_result: Result<(), Vec<String>>) {
    let problem = create_test_problem();
    let matrix = create_matrix_from_problem(&problem);
    let solution = create_test_solution(create_test_statistic(), stop_data);

    let result = check_routing(&CheckerContext::new(create_example_problem(), problem, Some(vec![matrix]), solution));

    assert_eq!(result, expected_result);
}

parameterized_test! {can_check_tour_statistic, (statistic, expected_result), {
    can_check_tour_statistic_impl(statistic, expected_result);
}}

can_check_tour_statistic! {
    case_01: (create_test_statistic(), Ok(())),

    case_02: (Statistic {
        distance: 1,
        ..create_test_statistic()
    }, Err(vec!["distance mismatch for tour statistic: my_vehicle_1, expected: '4', got: '1'".to_string()])),

    case_03: (Statistic {
        duration: 1,
        ..create_test_statistic()
    }, Err(vec!["duration mismatch for tour statistic: my_vehicle_1, expected: '6', got: '1'".to_string()])),
}

fn can_check_tour_statistic_impl(statistic: Statistic, expected_result: Result<(), Vec<String>>) {
    let problem = create_test_problem();
    let matrix = create_matrix_from_problem(&problem);
    let solution = create_test_solution(statistic, &[(1., 1), (3., 2), (6., 4)]);

    let result = check_routing(&CheckerContext::new(create_example_problem(), problem, Some(vec![matrix]), solution));

    assert_eq!(result, expected_result);
}

#[test]
fn can_check_solution_statistic() {
    let problem = create_test_problem();
    let matrix = create_matrix_from_problem(&problem);
    let solution = create_test_solution(create_test_statistic(), &[(1., 1), (3., 2), (6., 4)]);
    let wrong_statistic = Statistic { duration: 1, ..create_test_statistic() };
    let solution = Solution { statistic: wrong_statistic.clone(), ..solution };

    let result = check_routing(&CheckerContext::new(create_example_problem(), problem, Some(vec![matrix]), solution));

    assert_eq!(
        result,
        Err(vec![format!(
            "solution statistic mismatch, expected: '{:?}', got: '{:?}'",
            create_test_statistic(),
            wrong_statistic
        )])
    );
}
