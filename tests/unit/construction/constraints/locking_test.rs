use crate::construction::constraints::locking::LockingModule;
use crate::construction::constraints::RouteConstraintViolation;
use crate::construction::states::{RouteContext, RouteState};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::create_route_with_activities;
use crate::models::problem::Fleet;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition};
use std::sync::{Arc, RwLock};

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
        route: Arc::new(RwLock::new(create_route_with_activities(&fleet, used.as_str(), vec![]))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };
    let pipeline = create_constraint_pipeline_with_module(Box::new(LockingModule::new(&fleet, locks, 1)));

    let result = pipeline.evaluate_hard_route(&route_ctx, &job);

    assert_eq_option!(result, expected);
}
