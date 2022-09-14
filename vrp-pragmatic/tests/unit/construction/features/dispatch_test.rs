use super::*;
use crate::helpers::create_single_with_location;

const VIOLATION_CODE: ViolationCode = 1;

fn create_dispatch_job() -> Arc<Single> {
    let mut single = create_single_with_location(None);
    single.dimens.set_job_id("dispatch".to_string()).set_job_type("dispatch".to_string());

    Arc::new(single)
}

fn create_single() -> Arc<Single> {
    Arc::new(create_single_with_location(None))
}

parameterized_test! {can_skip_merge_dispatch, (source, candidate, expected), {
    can_skip_merge_dispatch_impl(Job::Single(source), Job::Single(candidate), expected);
}}

can_skip_merge_dispatch! {
    case_01: (create_single(), create_dispatch_job(), Err(VIOLATION_CODE)),
    case_02: (create_dispatch_job(), create_single(), Err(VIOLATION_CODE)),
    case_03: (create_single(), create_single(), Ok(())),
}

fn can_skip_merge_dispatch_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let constraint = create_dispatch_constraint(VIOLATION_CODE).unwrap().constraint.unwrap();

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
