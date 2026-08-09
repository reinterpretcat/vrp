use super::*;
use crate::construction::heuristics::{RegistryContext, RouteContext};
use crate::helpers::models::domain::{TestGoalContextBuilder, test_random};
use crate::helpers::models::problem::{FleetBuilder, TestSingleBuilder, test_driver, test_vehicle_with_id};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::problem::Fleet;
use crate::models::solution::Registry;
use std::collections::{HashMap, HashSet};

const VIOLATION_CODE: ViolationCode = ViolationCode(42);

#[test]
fn can_collect_missing_vehicle_ids() {
    let fleet = create_test_fleet(&["v1", "v2"]);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![("v1", 1), ("v2", 0)]);
    let feature = create_feature(vec![("v1", 1, false), ("v2", 1, false)]);

    feature.state.unwrap().accept_solution_state(&mut solution_ctx);

    let summary = solution_ctx.state.get_vehicle_shift_summary().unwrap();
    let expected = HashSet::from(["v2".to_string()]);

    assert_eq!(summary.missing_vehicle_ids, expected);
}

#[test]
fn can_block_insertions_on_satisfied_routes_when_missing_exists() {
    let fleet = create_test_fleet(&["v1", "v2"]);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![("v1", 1), ("v2", 0)]);
    let feature = create_feature(vec![("v1", 1, false), ("v2", 1, false)]);
    let constraint = feature.constraint.unwrap();
    feature.state.unwrap().accept_solution_state(&mut solution_ctx);
    let job = Job::Single(TestSingleBuilder::default().build_shared());

    let route_v1 = get_route_ctx(&solution_ctx, "v1");
    let violation = constraint.evaluate(&MoveContext::route(&solution_ctx, route_v1, &job));
    assert_eq!(violation, Some(ConstraintViolation { code: VIOLATION_CODE, stopped: false }));

    let route_v2 = get_route_ctx(&solution_ctx, "v2");
    let violation = constraint.evaluate(&MoveContext::route(&solution_ctx, route_v2, &job));
    assert_eq!(violation, None);
}

#[test]
fn allows_insertions_when_all_requirements_met() {
    let fleet = create_test_fleet(&["v1", "v2"]);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![("v1", 1), ("v2", 1)]);
    let feature = create_feature(vec![("v1", 1, false), ("v2", 1, false)]);
    let constraint = feature.constraint.unwrap();
    feature.state.unwrap().accept_solution_state(&mut solution_ctx);
    let job = Job::Single(TestSingleBuilder::default().build_shared());

    let route_v1 = get_route_ctx(&solution_ctx, "v1");
    let violation = constraint.evaluate(&MoveContext::route(&solution_ctx, route_v1, &job));
    assert_eq!(violation, None);
}

#[test]
fn can_allow_zero_usage() {
    let fleet = create_test_fleet(&["v1"]);
    let mut solution_ctx = create_solution_ctx(&fleet, vec![("v1", 0)]);
    let feature = create_feature(vec![("v1", 1, true)]);
    feature.state.unwrap().accept_solution_state(&mut solution_ctx);

    let summary = solution_ctx.state.get_vehicle_shift_summary().unwrap();
    assert!(summary.missing_vehicle_ids.is_empty());
}

fn create_feature(requirements: Vec<(&str, usize, bool)>) -> Feature {
    let requirements = requirements
        .into_iter()
        .map(|(id, value, allow_zero)| {
            (id.to_string(), MinShiftRequirement { minimum: value, allow_zero_usage: allow_zero })
        })
        .collect::<HashMap<_, _>>();

    MinVehicleShiftsFeatureBuilder::new("min_shifts")
        .with_violation_code(VIOLATION_CODE)
        .with_requirements(requirements)
        .build()
        .unwrap()
}

fn create_solution_ctx(fleet: &Fleet, vehicle_jobs: Vec<(&str, usize)>) -> SolutionContext {
    let routes = vehicle_jobs
        .into_iter()
        .map(|(vehicle_id, job_count)| {
            let mut route_builder = RouteBuilder::default();
            route_builder.with_vehicle(fleet, vehicle_id);
            if job_count > 0 {
                let activities = (0..job_count).map(|_| ActivityBuilder::default().build()).collect::<Vec<_>>();
                route_builder.add_activities(activities);
            }

            RouteContextBuilder::default().with_route(route_builder.build()).build()
        })
        .collect();

    SolutionContext {
        required: vec![],
        ignored: vec![],
        unassigned: Default::default(),
        locked: Default::default(),
        routes,
        registry: RegistryContext::new(&TestGoalContextBuilder::default().build(), Registry::new(fleet, test_random())),
        state: Default::default(),
    }
}

fn create_test_fleet(vehicle_ids: &[&str]) -> Fleet {
    let mut builder = FleetBuilder::default();
    builder.add_driver(test_driver());
    vehicle_ids.iter().for_each(|vehicle_id| {
        builder.add_vehicle(test_vehicle_with_id(vehicle_id));
    });

    builder.build()
}

fn get_route_ctx<'a>(solution_ctx: &'a SolutionContext, vehicle_id: &str) -> &'a RouteContext {
    solution_ctx
        .routes
        .iter()
        .find(|route_ctx| route_ctx.route().actor.vehicle.dimens.get_vehicle_id().unwrap() == vehicle_id)
        .unwrap()
}
