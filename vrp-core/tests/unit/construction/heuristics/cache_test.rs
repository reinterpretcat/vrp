use crate::construction::heuristics::cache::CacheContext;
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::helpers::models::domain::create_empty_insertion_context;
use crate::helpers::models::problem::SingleBuilder;
use crate::helpers::models::solution::create_empty_route_ctx;
use crate::models::problem::Job;

fn get_test_cache_data() -> (InsertionContext, RouteContext, CacheContext, Job) {
    let mut insertion_ctx = create_empty_insertion_context();
    CacheContext::inject(&mut insertion_ctx);
    let cache = CacheContext::from(&insertion_ctx);
    let route_ctx = create_empty_route_ctx();
    let job = SingleBuilder::default().build_as_job_ref();

    (insertion_ctx, route_ctx, cache, job)
}

#[test]
fn can_inject_and_get_cache() {
    let mut insertion_ctx = create_empty_insertion_context();

    CacheContext::inject(&mut insertion_ctx);
    let _ = CacheContext::from(&insertion_ctx);
}
/*
#[test]
fn can_insert_and_get_value() {
    let (_, route_ctx, cache, job) = get_test_cache_data();
    let eval_result = InsertionResult::make_failure_with_code(1, true, None);

    let actual_result = cache.evaluate_insertion(&route_ctx, &job, InsertionPosition::Any, {
        let eval_result = eval_result.clone();
        move || eval_result
    });
    let cached_result =
        cache.lookup.unwrap().get(&route_ctx, &job, &InsertionPosition::Any).expect("insertion result is not found");

    assert_eq!(actual_result.as_failure().unwrap().constraint, eval_result.as_failure().unwrap().constraint);
    assert_eq!(actual_result.as_failure().unwrap().constraint, cached_result.as_failure().unwrap().constraint);
}

#[test]
fn can_insert_and_remove_value() {
    let (_, route_ctx, cache, job) = get_test_cache_data();

    let _ = cache.evaluate_insertion(&route_ctx, &job, InsertionPosition::Any, || InsertionResult::make_failure());
    cache.accept_insertion(&route_ctx, &job);
    let cached_result = cache.lookup.unwrap().get(&route_ctx, &job, &InsertionPosition::Any);

    assert!(cached_result.is_none());
}
*/
