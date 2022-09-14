use super::*;
use crate::construction::heuristics::ActivityContext;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::{Job, Single};
use crate::models::solution::Activity;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition};
use std::sync::Arc;

const VIOLATION_CODE: ViolationCode = 1;

fn create_feature_constraint(fleet: &Fleet, locks: &[Arc<Lock>]) -> Arc<dyn FeatureConstraint + Send + Sync> {
    create_locked_jobs(fleet, locks, VIOLATION_CODE).unwrap().constraint.unwrap()
}

parameterized_test! {can_lock_jobs_to_actor, (used, locked, expected), {
    can_lock_jobs_to_actor_impl(used.to_string(), locked.to_string(), expected);
}}

can_lock_jobs_to_actor! {
    case01: ("v1", "v1", None),
    case02: ("v1", "v2", ConstraintViolation::fail(VIOLATION_CODE)),
}

fn can_lock_jobs_to_actor_impl(used: String, locked: String, expected: Option<ConstraintViolation>) {
    let job = Job::Single(test_single_with_id("s1"));
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();
    let locks = vec![Arc::new(Lock::new(
        Arc::new(move |actor| get_vehicle_id(actor.vehicle.as_ref()) == locked.as_str()),
        vec![LockDetail::new(LockOrder::Any, LockPosition::Any, vec![job.clone()])],
        false,
    ))];
    let solution_ctx = create_empty_solution_context();
    let route_ctx = create_route_context_with_activities(&fleet, used.as_str(), vec![]);
    let constraint = create_feature_constraint(&fleet, &locks);

    let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

    assert_eq!(result, expected);
}

fn stop() -> Option<ConstraintViolation> {
    Some(ConstraintViolation { code: 1, stopped: false })
}

fn some_activity() -> Activity {
    test_activity_with_location(1)
}

parameterized_test! {can_lock_jobs_to_position_in_tour, (position, activities_func, expected), {
    let s1 = test_single_with_id("s1");
    let s2 = test_single_with_id("s2");
    let activities = activities_func(s1.clone(), s2.clone());
    let jobs = vec![Job::Single(s1), Job::Single(s2)];

    can_lock_jobs_to_position_in_tour_impl(position, activities, jobs, expected);
}}

can_lock_jobs_to_position_in_tour! {
    case01_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_without_job(), test_activity_with_job(s1)),
        stop()),
    case02_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_without_job(), test_activity_with_job(s2)),
        stop()),
    case03_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s2), some_activity()),
        None),
    case04_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_with_job(s1), some_activity()),
        stop()),
    case05_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
    case06_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s2), test_activity_without_job()),
        None),
    case07_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_with_job(s1), test_activity_without_job()),
        stop()),

    case08_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_with_job(s1), test_activity_without_job()),
        stop()),
    case09_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_activity_with_job(s1)),
        None),
    case10_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), test_activity_with_job(s2)),
        stop()),
   case11_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
   case12_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_without_job(), test_activity_with_job(s1)),
        None),

  case13_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s1), test_activity_with_job(s2)),
        stop()),
  case14_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s2), test_activity_with_job(s1)),
        stop()),
  case15_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_activity_with_job(s1)),
        None),
  case16_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), test_activity_with_job(s2)),
        stop()),
  case17_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s2), some_activity()),
        None),
  case18_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s2), test_activity_without_job()),
        None),
  case19_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (test_activity_without_job(), test_activity_with_job(s1)),
        None),
  case20_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_activity_without_job(), test_activity_with_job(s2)),
        stop()),

  case21_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, s2: Arc<Single>| (test_activity_with_job(s1), test_activity_with_job(s2)),
       stop()),
  case22_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_activity_with_job(s1)),
       stop()),
}

fn can_lock_jobs_to_position_in_tour_impl(
    lock_position: LockPosition,
    activities: (Activity, Activity),
    jobs: Vec<Job>,
    expected: Option<ConstraintViolation>,
) {
    let (prev, next) = activities;
    let fleet = test_fleet();
    let locks = vec![Arc::new(Lock::new(
        Arc::new(|_| true),
        vec![LockDetail::new(LockOrder::Strict, lock_position, jobs)],
        false,
    ))];
    let constraint = create_feature_constraint(&fleet, &locks);

    let result = constraint.evaluate(&MoveContext::activity(
        &create_route_context_with_activities(&fleet, "v1", vec![]),
        &ActivityContext {
            index: 0,
            prev: &prev,
            target: &test_activity_with_job(test_single_with_id("new")),
            next: Some(&next),
        },
    ));

    assert_eq!(result, expected);
}

#[test]
fn can_handle_merge_locked_jobs() {
    let source = Job::Single(test_single_with_id("source"));
    let candidate1 = Job::Single(test_single_with_id("candidate1"));
    let candidate2 = Job::Single(test_single_with_id("candidate2"));
    let locks = vec![Arc::new(Lock::new(
        Arc::new(|_| true),
        vec![LockDetail::new(LockOrder::Strict, LockPosition::Any, vec![candidate1.clone()])],
        false,
    ))];

    let constraint = create_feature_constraint(&test_fleet(), &locks);

    assert!(constraint.merge(source.clone(), candidate1).is_err());
    assert!(constraint.merge(source, candidate2).is_ok());
}
