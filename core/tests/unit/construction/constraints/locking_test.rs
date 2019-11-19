use crate::construction::constraints::locking::StrictLockingModule;
use crate::construction::constraints::{ActivityConstraintViolation, RouteConstraintViolation};
use crate::construction::states::{ActivityContext, RouteContext, RouteState};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::{Fleet, Job};
use crate::models::solution::TourActivity;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition};
use std::sync::Arc;

parameterized_test! {can_lock_jobs_to_actor, (used, locked, expected), {
    can_lock_jobs_to_actor_impl(used.to_string(), locked.to_string(), expected);
}}

can_lock_jobs_to_actor! {
    case01: ("v1", "v1", None),
    case02: ("v1", "v2", Some(RouteConstraintViolation { code: 1 })),
}

fn can_lock_jobs_to_actor_impl(used: String, locked: String, expected: Option<RouteConstraintViolation>) {
    let job = Arc::new(test_single_job_with_id("s1"));
    let fleet = Fleet::new(vec![test_driver()], vec![test_vehicle_with_id("v1"), test_vehicle_with_id("v2")]);
    let locks = vec![Arc::new(Lock::new(
        Arc::new(move |actor| get_vehicle_id(actor.vehicle.as_ref()) == locked.as_str()),
        vec![LockDetail::new(LockOrder::Any, LockPosition::Any, vec![job.clone()])],
    ))];
    let route_ctx = RouteContext {
        route: Arc::new(create_route_with_activities(&fleet, used.as_str(), vec![])),
        state: Arc::new(RouteState::default()),
    };
    let pipeline = create_constraint_pipeline_with_module(Box::new(StrictLockingModule::new(&fleet, locks, 1)));

    let result = pipeline.evaluate_hard_route(&route_ctx, &job);

    assert_eq_option!(result, expected);
}

fn stop() -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code: 1, stopped: false })
}

fn some_activity() -> TourActivity {
    Box::new(test_activity_with_location(1))
}

parameterized_test! {can_lock_jobs_to_position_in_tour, (position, activities_func, expected), {
    let s1 = Arc::new(test_single_job_with_id("s1"));
    let s2 = Arc::new(test_single_job_with_id("s2"));
    let activities = activities_func(s1.clone(), s2.clone());
    let jobs = vec![s1.clone(), s2.clone()];

    can_lock_jobs_to_position_in_tour_impl(position, activities, jobs, expected);
}}

can_lock_jobs_to_position_in_tour! {
    case01_departure: (
        LockPosition::Departure,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        stop()),
    case02_departure: (
        LockPosition::Departure,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_without_job(), test_tour_activity_with_job(s2)),
        stop()),
    case03_departure: (
        LockPosition::Departure,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s2), some_activity()),
        None),
    case04_departure: (
        LockPosition::Departure,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_with_job(s1), some_activity()),
        stop()),
    case05_departure: (
        LockPosition::Departure,
        |_: Arc<Job>, _: Arc<Job>| (some_activity(), some_activity()),
        None),
    case06_departure: (
        LockPosition::Departure,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s2), test_tour_activity_without_job()),
        None),
    case07_departure: (
        LockPosition::Departure,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_with_job(s1), test_tour_activity_without_job()),
        stop()),

    case08_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_with_job(s1), test_tour_activity_without_job()),
        stop()),
    case09_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Job>, _: Arc<Job>| (some_activity(), test_tour_activity_with_job(s1)),
        None),
    case10_arrival: (
        LockPosition::Arrival,
        |_: Arc<Job>, s2: Arc<Job>| (some_activity(), test_tour_activity_with_job(s2)),
        stop()),
   case11_arrival: (
        LockPosition::Arrival,
        |_: Arc<Job>, _: Arc<Job>| (some_activity(), some_activity()),
        None),
   case12_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        None),

  case13_any: (
        LockPosition::Any,
        |s1: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s1), test_tour_activity_with_job(s2)),
        stop()),
  case14_any: (
        LockPosition::Any,
        |s1: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s2), test_tour_activity_with_job(s1)),
        stop()),
  case15_any: (
        LockPosition::Any,
        |s1: Arc<Job>, _: Arc<Job>| (some_activity(), test_tour_activity_with_job(s1)),
        None),
  case16_any: (
        LockPosition::Any,
        |_: Arc<Job>, s2: Arc<Job>| (some_activity(), test_tour_activity_with_job(s2)),
        stop()),
  case17_any: (
        LockPosition::Any,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s2), some_activity()),
        None),
  case18_any: (
        LockPosition::Any,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s2), test_tour_activity_without_job()),
        None),
  case19_any: (
        LockPosition::Any,
        |s1: Arc<Job>, _: Arc<Job>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        None),
  case20_any: (
        LockPosition::Any,
        |_: Arc<Job>, s2: Arc<Job>| (test_tour_activity_without_job(), test_tour_activity_with_job(s2)),
        stop()),

  case21_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Job>, s2: Arc<Job>| (test_tour_activity_with_job(s1), test_tour_activity_with_job(s2)),
       stop()),
  case22_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Job>, _: Arc<Job>| (some_activity(), test_tour_activity_with_job(s1)),
       stop()),
}

fn can_lock_jobs_to_position_in_tour_impl(
    lock_position: LockPosition,
    activities: (TourActivity, TourActivity),
    jobs: Vec<Arc<Job>>,
    expected: Option<ActivityConstraintViolation>,
) {
    let (prev, next) = activities;
    let fleet = Fleet::new(vec![test_driver()], vec![test_vehicle_with_id("v1")]);
    let locks =
        vec![Arc::new(Lock::new(Arc::new(|_| true), vec![LockDetail::new(LockOrder::Strict, lock_position, jobs)]))];
    let pipeline = create_constraint_pipeline_with_module(Box::new(StrictLockingModule::new(&fleet, locks, 1)));

    let result = pipeline.evaluate_hard_activity(
        &RouteContext {
            route: Arc::new(create_route_with_activities(&fleet, "v1", vec![])),
            state: Arc::new(RouteState::default()),
        },
        &ActivityContext {
            index: 0,
            prev: &prev,
            target: &test_tour_activity_with_job(Arc::new(test_single_job_with_id("new"))),
            next: Some(&next),
        },
    );

    assert_eq_option!(result, expected);
}
