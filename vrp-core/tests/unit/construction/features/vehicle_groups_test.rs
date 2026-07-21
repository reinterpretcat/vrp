use super::*;
use crate::construction::enablers::create_typed_actor_groups;
use crate::helpers::models::domain::{TestGoalContextBuilder, test_random};
use crate::helpers::models::problem::{FleetBuilder, TestSingleBuilder, test_driver, test_vehicle_with_id};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder, RouteStateBuilder};
use crate::models::problem::{Actor, Fleet, Single};
use crate::models::solution::Registry;
use std::collections::HashSet;
use std::sync::Arc;

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

fn create_test_feature(total_jobs: usize) -> Feature {
    create_vehicle_group_feature("vehicle_group", total_jobs, VIOLATION_CODE).unwrap()
}

// Two actors on the SAME vehicle id "v1" (two shifts of one employee), plus "v2".
fn create_test_fleet() -> Fleet {
    FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v1"))
        .add_vehicle(test_vehicle_with_id("v2"))
        .with_group_key_fn(Box::new(|actors| {
            Box::new(create_typed_actor_groups(actors, |a| a.vehicle.dimens.get_vehicle_id().cloned().unwrap()))
        }))
        .build()
}

fn create_test_single(group: Option<&str>) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();
    if let Some(group) = group {
        builder.dimens_mut().set_vehicle_group(group.to_string());
    }
    builder.build_shared()
}

// nth actor whose vehicle id == `vehicle` (0-based), so two "v1" shifts are addressable.
fn nth_actor(fleet: &Fleet, vehicle: &str, nth: usize) -> Arc<Actor> {
    fleet.actors.iter().filter(|a| a.vehicle.dimens.get_vehicle_id().unwrap() == vehicle).nth(nth).unwrap().clone()
}

fn solution_ctx(total_jobs: usize, fleet: &Fleet, routes: Vec<(Arc<Actor>, Vec<Option<&str>>)>) -> SolutionContext {
    // `total_jobs` counts every job in the whole problem, including ones already
    // placed on a route below, so the still-unassigned `required` set is the remainder.
    let already_assigned = routes.iter().map(|(_, groups)| groups.len()).sum::<usize>();
    SolutionContext {
        required: (0..(total_jobs - already_assigned)).map(|_| Job::Single(create_test_single(None))).collect(),
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes: routes
            .into_iter()
            .map(|(actor, groups)| {
                RouteContextBuilder::default()
                    .with_state(
                        RouteStateBuilder::default()
                            .set_route_state(|state| {
                                state.set_current_vehicle_groups(
                                    groups.iter().filter_map(|g| *g).map(|g| g.to_string()).collect::<HashSet<_>>(),
                                )
                            })
                            .build(),
                    )
                    .with_route(
                        RouteBuilder::default()
                            .with_actor(actor)
                            .add_activities(
                                groups.into_iter().map(|g| {
                                    ActivityBuilder::with_location(1).job(Some(create_test_single(g))).build()
                                }),
                            )
                            .build(),
                    )
                    .build()
            })
            .collect(),
        registry: RegistryContext::new(&TestGoalContextBuilder::default().build(), Registry::new(fleet, test_random())),
        state: Default::default(),
    }
}

#[test]
fn same_vehicle_across_shifts_is_allowed() {
    let fleet = create_test_fleet();
    let (a, b) = (nth_actor(&fleet, "v1", 0), nth_actor(&fleet, "v1", 1)); // two shifts, one vehicle
    let total = 3;
    let ctx = solution_ctx(total, &fleet, vec![(a.clone(), vec![]), (b, vec![Some("g1")])]);
    let route_ctx = ctx.routes.first().unwrap();
    let constraint = create_test_feature(total).constraint.unwrap();
    let job = Job::Single(create_test_single(Some("g1")));
    // g1 already on the other shift of the SAME vehicle → no violation.
    assert_eq!(constraint.evaluate(&MoveContext::route(&ctx, route_ctx, &job)), None);
}

#[test]
fn different_vehicle_is_rejected() {
    let fleet = create_test_fleet();
    let (v1, v2) = (nth_actor(&fleet, "v1", 0), nth_actor(&fleet, "v2", 0));
    let total = 3;
    let ctx = solution_ctx(total, &fleet, vec![(v1, vec![]), (v2, vec![Some("g1")])]);
    let route_ctx = ctx.routes.first().unwrap(); // inserting into v1 while g1 is on v2
    let constraint = create_test_feature(total).constraint.unwrap();
    let job = Job::Single(create_test_single(Some("g1")));
    assert_eq!(
        constraint.evaluate(&MoveContext::route(&ctx, route_ctx, &job)),
        Some(ConstraintViolation { code: VIOLATION_CODE, stopped: true })
    );
}
