use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{FleetBuilder, TestSingleBuilder, TestVehicleBuilder, test_driver};
use crate::helpers::models::solution::{RouteBuilder, RouteContextBuilder};
use crate::construction::features::{create_tags_feature, JobTagsDimension, VehicleTagsDimension};
use crate::construction::heuristics::MoveContext;
use crate::models::problem::{Job, Vehicle};
use crate::models::ViolationCode;
use std::collections::HashSet;

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

fn create_job_with_tags(tags: Option<Vec<&str>>) -> Job {
    let mut builder = TestSingleBuilder::default();

    if let Some(tags) = tags {
        let tag_set: HashSet<String> = HashSet::from_iter(tags.iter().map(|s| s.to_string()));
        builder.dimens_mut().set_job_tags(tag_set);
    }

    builder.build_as_job_ref()
}

fn create_vehicle_with_tags(tags: Option<Vec<&str>>) -> Vehicle {
    let mut builder = TestVehicleBuilder::default();

    if let Some(tags) = tags {
        let tag_set: HashSet<String> = HashSet::from_iter(tags.iter().map(|s| s.to_string()));
        builder.dimens_mut().set_vehicle_tags(tag_set);
    }

    builder.id("v1").build()
}

#[test]
fn can_create_tags_feature() {
    let feature = create_tags_feature("tags", VIOLATION_CODE).unwrap();

    assert_eq!(feature.name, "tags");
    assert!(feature.constraint.is_some());
}

#[test]
fn job_without_tags_passes_any_vehicle() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(create_vehicle_with_tags(None))
        .build();
    let route_ctx =
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build();

    let constraint = create_tags_feature("tags", VIOLATION_CODE).unwrap().constraint.unwrap();

    let actual = constraint.evaluate(&MoveContext::route(
        &TestInsertionContextBuilder::default().build().solution,
        &route_ctx,
        &create_job_with_tags(None),
    ));

    assert_eq!(actual, None);
}

#[test]
fn job_with_tags_fails_without_vehicle_tags() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(create_vehicle_with_tags(None))
        .build();
    let route_ctx =
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build();

    let constraint = create_tags_feature("tags", VIOLATION_CODE).unwrap().constraint.unwrap();

    let actual = constraint.evaluate(&MoveContext::route(
        &TestInsertionContextBuilder::default().build().solution,
        &route_ctx,
        &create_job_with_tags(Some(vec!["fragile"])),
    ));

    assert!(actual.is_some());
}

#[test]
fn job_with_tags_succeeds_with_matching_vehicle_tags() {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicle(create_vehicle_with_tags(Some(vec!["fragile"])))
        .build();
    let route_ctx =
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build();

    let constraint = create_tags_feature("tags", VIOLATION_CODE).unwrap().constraint.unwrap();

    let actual = constraint.evaluate(&MoveContext::route(
        &TestInsertionContextBuilder::default().build().solution,
        &route_ctx,
        &create_job_with_tags(Some(vec!["fragile"])),
    ));

    assert_eq!(actual, None);
}