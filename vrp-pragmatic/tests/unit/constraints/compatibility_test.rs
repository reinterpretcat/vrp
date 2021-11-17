use super::*;
use crate::helpers::*;
use vrp_core::construction::heuristics::*;
use vrp_core::models::problem::*;

const VIOLATION_CODE: i32 = 1;
const STATE_KEY: i32 = 2;

fn create_test_single(compatibility: Option<String>) -> Arc<Single> {
    let mut single = create_single_with_location(Some(DEFAULT_JOB_LOCATION));
    if let Some(compatibility) = compatibility {
        single.dimens.set_value("compat", compatibility)
    }

    Arc::new(single)
}

fn create_test_route_ctx(compatibility: Option<String>) -> RouteContext {
    let mut state = RouteState::default();
    state.put_route_state(STATE_KEY, compatibility.clone());

    RouteContext::new_with_state(
        Arc::new(create_route_with_activities(
            &test_fleet(),
            "v1",
            vec![create_activity_with_job_at_location(create_test_single(compatibility), 1)],
        )),
        Arc::new(state),
    )
}

parameterized_test! {can_use_compatibility, (job_compat, route_compat, expected), {
    can_use_compatibility_impl(job_compat, route_compat, expected);
}}

can_use_compatibility! {
    case_01: (Some("junk"), Some("food"), Some(())),
    case_02: (Some("junk"), None, None),
    case_03: (None, Some("junk"), None),
    case_04: (Some("food"), Some("food"), None),
}

fn can_use_compatibility_impl(job_compat: Option<&str>, route_compat: Option<&str>, expected: Option<()>) {
    let solution_ctx = create_solution_context_for_fleet(&test_fleet());
    let route_ctx = create_test_route_ctx(route_compat.map(|v| v.to_string()));
    let job = Job::Single(create_test_single(job_compat.map(|v| v.to_string())));

    let result = CompatibilityHardRouteConstraint { code: VIOLATION_CODE, state_key: STATE_KEY }
        .evaluate_job(&solution_ctx, &route_ctx, &job)
        .map(|_| ());

    assert_eq!(result, expected);
}

parameterized_test! {can_accept_route_state, (route_compat, expected), {
    can_accept_route_state_impl(route_compat, expected);
}}

can_accept_route_state! {
    case_01: (Some("junk"), Some(Some("junk"))),
    case_02: (None, Some(None)),
}

fn can_accept_route_state_impl(route_compat: Option<&str>, expected: Option<Option<&str>>) {
    let expected = expected.map(|v| v.map(|v| v.to_string()));
    let mut route_ctx = create_test_route_ctx(route_compat.map(|v| v.to_string()));
    let compatibility = CompatibilityModule::new(VIOLATION_CODE, STATE_KEY);

    compatibility.accept_route_state(&mut route_ctx);

    let result = route_ctx.state.get_route_state::<Option<String>>(STATE_KEY).cloned();
    assert_eq!(result, expected);
}

parameterized_test! {can_merge_jobs, (source_compat, candidate_compat, expected), {
    can_merge_jobs_impl(source_compat, candidate_compat, expected);
}}

can_merge_jobs! {
    case_01: (Some("junk"), Some("junk"), Ok(Some("junk".to_string()))),
    case_02: (Some("junk"), Some("food"), Err(VIOLATION_CODE)),
    case_03: (Some("food"), Some("junk"), Err(VIOLATION_CODE)),
    case_04: (None, None, Ok(None)),
}

fn can_merge_jobs_impl(
    source_compat: Option<&str>,
    candidate_compat: Option<&str>,
    expected: Result<Option<String>, i32>,
) {
    let source = Job::Single(create_test_single(source_compat.map(|v| v.to_string())));
    let candidate = Job::Single(create_test_single(candidate_compat.map(|v| v.to_string())));
    let compatibility = CompatibilityModule::new(VIOLATION_CODE, STATE_KEY);

    let result = compatibility.merge(source, candidate).map(|job| get_job_compatibility(&job).cloned());

    match (result, expected) {
        (Ok(_), Err(_)) => unreachable!("unexpected err result"),
        (Err(_), Ok(_)) => unreachable!("unexpected ok result"),
        (Err(res_code), Err(exp_code)) => assert_eq!(res_code, exp_code),
        (Ok(result), Ok(expected)) => assert_eq!(result, expected),
    }
}
