use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder, RouteStateBuilder};
use std::sync::Arc;

const VIOLATION_CODE: i32 = 1;
const DEFAULT_JOB_LOCATION: Location = 1;

fn create_feature() -> Feature {
    create_compatibility_feature("compatibility", VIOLATION_CODE).unwrap()
}

fn create_test_single(compatibility: Option<String>) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();

    if let Some(compatibility) = compatibility {
        builder.dimens_mut().set_job_compatibility(compatibility);
    }

    builder.location(Some(DEFAULT_JOB_LOCATION)).build_shared()
}

fn create_test_route_ctx(compatibility: Option<String>) -> RouteContext {
    RouteContextBuilder::default()
        .with_route(
            RouteBuilder::with_default_vehicle()
                .add_activity(
                    ActivityBuilder::with_location(1).job(Some(create_test_single(compatibility.clone()))).build(),
                )
                .build(),
        )
        .with_state(
            RouteStateBuilder::default()
                .set_route_state(|state| {
                    if let Some(compatibility) = &compatibility {
                        state.set_current_compatibility(compatibility.clone())
                    }
                })
                .build(),
        )
        .build()
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
    let solution_ctx = TestInsertionContextBuilder::default()
        .with_routes(vec![create_test_route_ctx(route_compat.map(|v| v.to_string()))])
        .build()
        .solution;
    let job = Job::Single(create_test_single(job_compat.map(|v| v.to_string())));

    let result = create_feature()
        .constraint
        .unwrap()
        .evaluate(&MoveContext::route(&solution_ctx, &solution_ctx.routes[0], &job))
        .map(|_| ());

    assert_eq!(result, expected);
}

parameterized_test! {can_accept_route_state, (route_compat, expected), {
    can_accept_route_state_impl(route_compat, expected);
}}

can_accept_route_state! {
    case_01: (Some("junk"), Some("junk")),
    case_02: (None, None),
}

fn can_accept_route_state_impl(route_compat: Option<&str>, expected: Option<&str>) {
    let expected = expected.map(|v| v.to_string());
    let mut route_ctx = create_test_route_ctx(route_compat.map(|v| v.to_string()));
    let state = create_feature().state.unwrap();

    state.accept_route_state(&mut route_ctx);

    let result = route_ctx.state().get_current_compatibility().cloned();
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
    let constraint = create_feature().constraint.unwrap();

    let result = constraint.merge(source, candidate).map(|job| job.dimens().get_job_compatibility().cloned());

    match (result, expected) {
        (Ok(_), Err(_)) => unreachable!("unexpected err result"),
        (Err(_), Ok(_)) => unreachable!("unexpected ok result"),
        (Err(res_code), Err(exp_code)) => assert_eq!(res_code, exp_code),
        (Ok(result), Ok(expected)) => assert_eq!(result, expected),
    }
}
