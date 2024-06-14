use super::*;
use crate::helpers::construction::heuristics::InsertionContextBuilder;
use crate::helpers::models::problem::SingleBuilder;
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder, RouteStateBuilder};
use std::sync::Arc;

const VIOLATION_CODE: i32 = 1;
const DEFAULT_JOB_LOCATION: Location = 1;

#[derive(Clone)]
struct TestCompatibilityAspects {
    state_key: StateKey,
}

struct JobCompatDimenKey;

impl CompatibilityAspects for TestCompatibilityAspects {
    fn get_job_compatibility<'a>(&self, job: &'a Job) -> Option<&'a String> {
        job.dimens().get_value::<JobCompatDimenKey, _>()
    }

    fn get_state_key(&self) -> StateKey {
        self.state_key
    }

    fn get_violation_code(&self) -> ViolationCode {
        VIOLATION_CODE
    }
}

fn create_feature(state_key: StateKey) -> Feature {
    create_compatibility_feature("compatibility", TestCompatibilityAspects { state_key }).unwrap()
}

fn create_test_single(compatibility: Option<String>) -> Arc<Single> {
    let mut builder = SingleBuilder::default();

    if let Some(compatibility) = compatibility {
        builder.property::<JobCompatDimenKey, _>(compatibility);
    }

    builder.location(Some(DEFAULT_JOB_LOCATION)).build_shared()
}

fn create_test_route_ctx(compatibility: Option<String>) -> RouteContext {
    let state_key = StateKeyRegistry::default().next_key();
    RouteContextBuilder::default()
        .with_route(
            RouteBuilder::with_default_vehicle()
                .add_activity(
                    ActivityBuilder::with_location(1).job(Some(create_test_single(compatibility.clone()))).build(),
                )
                .build(),
        )
        .with_state(RouteStateBuilder::default().add_route_state(state_key, compatibility).build())
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
    let state_key = StateKeyRegistry::default().next_key();
    let solution_ctx = InsertionContextBuilder::default()
        .with_routes(vec![create_test_route_ctx(route_compat.map(|v| v.to_string()))])
        .build()
        .solution;
    let job = Job::Single(create_test_single(job_compat.map(|v| v.to_string())));

    let result = create_feature(state_key)
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
    case_01: (Some("junk"), Some(Some("junk"))),
    case_02: (None, Some(None)),
}

fn can_accept_route_state_impl(route_compat: Option<&str>, expected: Option<Option<&str>>) {
    let state_key = StateKeyRegistry::default().next_key();
    let expected = expected.map(|v| v.map(|v| v.to_string()));
    let mut route_ctx = create_test_route_ctx(route_compat.map(|v| v.to_string()));
    let state = create_feature(state_key).state.unwrap();

    state.accept_route_state(&mut route_ctx);

    let result = route_ctx.state().get_route_state::<Option<String>>(state_key).cloned();
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
    let state_key = StateKeyRegistry::default().next_key();
    let source = Job::Single(create_test_single(source_compat.map(|v| v.to_string())));
    let candidate = Job::Single(create_test_single(candidate_compat.map(|v| v.to_string())));
    let constraint = create_feature(state_key).constraint.unwrap();

    let result =
        constraint.merge(source, candidate).map(|job| job.dimens().get_value::<JobCompatDimenKey, _>().cloned());

    match (result, expected) {
        (Ok(_), Err(_)) => unreachable!("unexpected err result"),
        (Err(_), Ok(_)) => unreachable!("unexpected ok result"),
        (Err(res_code), Err(exp_code)) => assert_eq!(res_code, exp_code),
        (Ok(result), Ok(expected)) => assert_eq!(result, expected),
    }
}
