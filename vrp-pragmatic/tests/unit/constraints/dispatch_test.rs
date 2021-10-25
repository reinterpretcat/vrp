use super::*;
use crate::helpers::create_single_with_location;

fn create_dispatch() -> Arc<Single> {
    let mut single = create_single_with_location(None);
    single.dimens.set_id("dispatch");
    single.dimens.set_value("type", "dispatch".to_string());

    Arc::new(single)
}

fn create_single() -> Arc<Single> {
    Arc::new(create_single_with_location(None))
}

parameterized_test! {can_skip_merge_dispatch, (source, candidate, expected), {
    can_skip_merge_dispatch_impl(Job::Single(source), Job::Single(candidate), expected);
}}

can_skip_merge_dispatch! {
    case_01: (create_single(), create_dispatch(), Err(0)),
    case_02: (create_dispatch(), create_single(), Err(0)),
    case_03: (create_single(), create_single(), Ok(())),
}

fn can_skip_merge_dispatch_impl(source: Job, candidate: Job, expected: Result<(), i32>) {
    let constraint = DispatchModule::new(0);

    let result = constraint.merge(source, candidate).map(|_| ());

    assert_eq!(result, expected);
}
