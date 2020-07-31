use super::*;
use crate::helpers::*;

fn create_test_problem(limits: Option<VehicleLimits>) -> Problem {
    Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType { limits, ..create_default_vehicle_type() }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    }
}

fn create_test_solution(statistic: Statistic) -> Solution {
    Solution {
        tours: vec![Tour {
            vehicle_id: "my_vehicle_1".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![],
            statistic,
            ..create_empty_tour()
        }],
        ..create_empty_solution()
    }
}

parameterized_test! {can_check_shift_and_distance_limit, (max_distance, shift_time, actual, expected_result), {
    let expected_result = if let Err(prefix_msg) = expected_result {
        Err(format!(
            "{} violation, expected: not more than {}, got: {}, vehicle id 'my_vehicle_1', shift index: 0",
            prefix_msg, max_distance.unwrap_or_else(|| shift_time.unwrap()), actual,
        ))
    } else {
        Ok(())
    };
    can_check_shift_and_distance_limit_impl(max_distance, shift_time, actual, expected_result);
}}

can_check_shift_and_distance_limit! {
    case_01: (Some(10.), None, 11, Result::<(), _>::Err("max distance limit")),
    case_02: (Some(10.), None, 10, Result::<_, &str>::Ok(())),
    case_03: (Some(10.), None, 9, Result::<_, &str>::Ok(())),

    case_04: (None, Some(10.), 11, Result::<(), _>::Err("shift time limit")),
    case_05: (None, Some(10.), 10, Result::<_, &str>::Ok(())),
    case_06: (None, Some(10.), 9, Result::<_, &str>::Ok(())),

    case_07: (None, None, i64::max_value(), Result::<_, &str>::Ok(())),
}

pub fn can_check_shift_and_distance_limit_impl(
    max_distance: Option<f64>,
    shift_time: Option<f64>,
    actual: i64,
    expected: Result<(), String>,
) {
    let problem = create_test_problem(Some(VehicleLimits { max_distance, shift_time, allowed_areas: None }));
    let solution = create_test_solution(Statistic { distance: actual, duration: actual, ..Statistic::default() });

    let result = check_limits(&CheckerContext::new(problem, None, solution));

    assert_eq!(result, expected);
}
