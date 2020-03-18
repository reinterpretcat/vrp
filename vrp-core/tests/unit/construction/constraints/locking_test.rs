use crate::construction::constraints::locking::StrictLockingModule;
use crate::construction::constraints::{ActivityConstraintViolation, RouteConstraintViolation};
use crate::construction::states::ActivityContext;
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::{Job, Single};
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
    let job = Job::Single(test_single_with_id("s1"));
    let fleet = FleetBuilder::new()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .build();
    let locks = vec![Arc::new(Lock::new(
        Arc::new(move |actor| get_vehicle_id(actor.vehicle.as_ref()) == locked.as_str()),
        vec![LockDetail::new(LockOrder::Any, LockPosition::Any, vec![job.clone()])],
    ))];
    let solution_ctx = create_empty_solution_context();
    let route_ctx = create_route_context_with_activities(&fleet, used.as_str(), vec![]);
    let pipeline = create_constraint_pipeline_with_module(Box::new(StrictLockingModule::new(&fleet, locks, 1)));

    let result = pipeline.evaluate_hard_route(&solution_ctx, &route_ctx, &job);

    assert_eq_option!(result, expected);
}

fn stop() -> Option<ActivityConstraintViolation> {
    Some(ActivityConstraintViolation { code: 1, stopped: false })
}

fn some_activity() -> TourActivity {
    Box::new(test_activity_with_location(1))
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
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        stop()),
    case02_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_without_job(), test_tour_activity_with_job(s2)),
        stop()),
    case03_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s2), some_activity()),
        None),
    case04_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_with_job(s1), some_activity()),
        stop()),
    case05_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
    case06_departure: (
        LockPosition::Departure,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s2), test_tour_activity_without_job()),
        None),
    case07_departure: (
        LockPosition::Departure,
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_with_job(s1), test_tour_activity_without_job()),
        stop()),

    case08_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_with_job(s1), test_tour_activity_without_job()),
        stop()),
    case09_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_tour_activity_with_job(s1)),
        None),
    case10_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), test_tour_activity_with_job(s2)),
        stop()),
   case11_arrival: (
        LockPosition::Arrival,
        |_: Arc<Single>, _: Arc<Single>| (some_activity(), some_activity()),
        None),
   case12_arrival: (
        LockPosition::Arrival,
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        None),

  case13_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s1), test_tour_activity_with_job(s2)),
        stop()),
  case14_any: (
        LockPosition::Any,
        |s1: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s2), test_tour_activity_with_job(s1)),
        stop()),
  case15_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_tour_activity_with_job(s1)),
        None),
  case16_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (some_activity(), test_tour_activity_with_job(s2)),
        stop()),
  case17_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s2), some_activity()),
        None),
  case18_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s2), test_tour_activity_without_job()),
        None),
  case19_any: (
        LockPosition::Any,
        |s1: Arc<Single>, _: Arc<Single>| (test_tour_activity_without_job(), test_tour_activity_with_job(s1)),
        None),
  case20_any: (
        LockPosition::Any,
        |_: Arc<Single>, s2: Arc<Single>| (test_tour_activity_without_job(), test_tour_activity_with_job(s2)),
        stop()),

  case21_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, s2: Arc<Single>| (test_tour_activity_with_job(s1), test_tour_activity_with_job(s2)),
       stop()),
  case22_fixed: (
       LockPosition::Fixed,
       |s1: Arc<Single>, _: Arc<Single>| (some_activity(), test_tour_activity_with_job(s1)),
       stop()),
}

fn can_lock_jobs_to_position_in_tour_impl(
    lock_position: LockPosition,
    activities: (TourActivity, TourActivity),
    jobs: Vec<Job>,
    expected: Option<ActivityConstraintViolation>,
) {
    let (prev, next) = activities;
    let fleet = FleetBuilder::new().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
    let locks =
        vec![Arc::new(Lock::new(Arc::new(|_| true), vec![LockDetail::new(LockOrder::Strict, lock_position, jobs)]))];
    let pipeline = create_constraint_pipeline_with_module(Box::new(StrictLockingModule::new(&fleet, locks, 1)));

    let result = pipeline.evaluate_hard_activity(
        &create_route_context_with_activities(&fleet, "v1", vec![]),
        &ActivityContext {
            index: 0,
            prev: &prev,
            target: &test_tour_activity_with_job(test_single_with_id("new")),
            next: Some(&next),
        },
    );

    assert_eq_option!(result, expected);
}
