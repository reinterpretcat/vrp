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
    create_locked_jobs_feature("locked_jobs", fleet, locks, VIOLATION_CODE).unwrap().constraint.unwrap()
}

parameterized_test! {can_lock_jobs_to_actor, (used, locked, expected), {
    can_lock_jobs_to_actor_impl(used.to_string(), locked.to_string(), expected);
}}

can_lock_jobs_to_actor! {
    case01: ("v1", "v1", None),
    case02: ("v1", "v2", ConstraintViolation::fail(VIOLATION_CODE)),
}

fn can_lock_jobs_to_actor_impl(used: String, locked: String, expected: Option<ConstraintViolation>) {
    let job = SingleBuilder::default().id("s1").build_as_job_ref();
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
    let route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, used.as_str()).build())
        .build();
    let constraint = create_feature_constraint(&fleet, &locks);

    let result = constraint.evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

    assert_eq!(result, expected);
}

fn stop() -> Option<ConstraintViolation> {
    Some(ConstraintViolation { code: 1, stopped: false })
}

fn some_activity() -> Activity {
    ActivityBuilder::with_location(1).build()
}

parameterized_test! {can_lock_jobs_to_position_in_tour, (position, activities_func, expected), {
    let s1 = SingleBuilder::default().id("s1").build_shared();
    let s2 = SingleBuilder::default().id("s2").build_shared();
    let activities = activities_func(s1.clone(), s2.clone());
    let jobs = vec![Job::Single(s1), Job::Single(s2)];

    can_lock_jobs_to_position_in_tour_impl(position, activities, jobs, expected);
}}

can_lock_jobs_to_position_in_tour! {
    case01_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(None).build(),  ActivityBuilder::default().job(Some(s1)).build()),
        stop()),
    case02_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(None).build(), ActivityBuilder::default().job(Some(s2)).build()),
        stop()),
    case03_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s2)).build(), some_activity()),
        None),
    case04_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(Some(s1)).build(), some_activity()),
        stop()),
    case05_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
    case06_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s2)).build(), ActivityBuilder::default().job(None).build()),
        None),
    case07_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(Some(s1)).build(), ActivityBuilder::default().job(None).build()),
        stop()),

    case08_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(Some(s1)).build(), ActivityBuilder::default().job(None).build()),
        stop()),
    case09_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), ActivityBuilder::default().job(Some(s1)).build()),
        None),
    case10_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), ActivityBuilder::default().job(Some(s2)).build()),
        stop()),
   case11_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
   case12_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(None).build(), ActivityBuilder::default().job(Some(s1)).build()),
        None),

  case13_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s1)).build(), ActivityBuilder::default().job(Some(s2)).build()),
        stop()),
  case14_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s2)).build(), ActivityBuilder::default().job(Some(s1)).build()),
        stop()),
  case15_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), ActivityBuilder::default().job(Some(s1)).build()),
        None),
  case16_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), ActivityBuilder::default().job(Some(s2)).build()),
        stop()),
  case17_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s2)).build(), some_activity()),
        None),
  case18_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s2)).build(), ActivityBuilder::default().job(None).build()),
        None),
  case19_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (ActivityBuilder::default().job(None).build(), ActivityBuilder::default().job(Some(s1)).build()),
        None),
  case20_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(None).build(), ActivityBuilder::default().job(Some(s2)).build()),
        stop()),

  case21_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, s2: Arc<Single>| (ActivityBuilder::default().job(Some(s1)).build(), ActivityBuilder::default().job(Some(s2)).build()),
       stop()),
  case22_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, _: Arc<Single>| (some_activity(), ActivityBuilder::default().job(Some(s1)).build()),
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
        &RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build(),
        &ActivityContext {
            index: 0,
            prev: &prev,
            target: &ActivityBuilder::default().job(Some(SingleBuilder::default().id("new").build_shared())).build(),
            next: Some(&next),
        },
    ));

    assert_eq!(result, expected);
}

#[test]
fn can_handle_merge_locked_jobs() {
    let source = SingleBuilder::default().id("source").build_as_job_ref();
    let candidate1 = SingleBuilder::default().id("candidate1").build_as_job_ref();
    let candidate2 = SingleBuilder::default().id("candidate2").build_as_job_ref();
    let locks = vec![Arc::new(Lock::new(
        Arc::new(|_| true),
        vec![LockDetail::new(LockOrder::Strict, LockPosition::Any, vec![candidate1.clone()])],
        false,
    ))];

    let constraint = create_feature_constraint(&test_fleet(), &locks);

    assert!(constraint.merge(source.clone(), candidate1).is_err());
    assert!(constraint.merge(source, candidate2).is_ok());
}
