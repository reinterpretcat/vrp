use crate::construction::features::{TerritoryBalance, TerritoryFeatureBuilder, TerritoryProximity};
use crate::construction::heuristics::{InsertionContext, RouteContext, RouteState};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::test_logger;
use crate::helpers::models::problem::{
    FleetBuilder, TestSingleBuilder, TestTransportCost, TestVehicleBuilder, get_test_actor_from_fleet, test_driver,
};
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::Feature;
use crate::models::common::TimeInterval;
use crate::models::problem::{Actor, DriverIdDimension, Job, Jobs, Single, VehicleDetail, VehiclePlace};
use crate::models::solution::{Route, Tour};
use std::collections::HashMap;
use std::sync::Arc;

/// The two insertion contexts a territory fixture builds: `correct_assignment` has every job on
/// its nearest anchor's driver; `swapped_assignment` swaps the two jobs onto the far driver so
/// PULL has something to penalize.
struct TerritoryFixtureContexts {
    correct_assignment: InsertionContext,
    swapped_assignment: InsertionContext,
}

fn build_vehicle(id: &str, driver_id: &str) -> crate::models::problem::Vehicle {
    let mut builder = TestVehicleBuilder::default();
    builder.id(id).details(vec![VehicleDetail {
        start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
        end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
    }]);
    builder.dimens_mut().set_driver_id(driver_id.to_string());
    builder.build()
}

fn route_with(actor: Arc<Actor>, job: Arc<Single>, job_location: usize) -> RouteContext {
    let route = Route {
        actor,
        tour: {
            let mut tour = Tour::default();
            tour.set_start(ActivityBuilder::with_location(0).job(None).build());
            tour.set_end(ActivityBuilder::with_location(0).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(job_location).job(Some(job)).build());
            tour
        },
    };
    RouteContext::new_with_state(route, RouteState::default())
}

/// Builds a territory feature plus two insertion contexts over a fixed two-driver, two-job
/// scenario: driver "d0" anchored at location 0, driver "d1" anchored at location 100; job
/// "near" at location 5 (closest to d0's anchor) and job "far" at location 95 (closest to d1's
/// anchor). `correct_assignment` puts each job on its nearest driver; `swapped_assignment` puts
/// each job on the other (far) driver.
fn territory_fixture(
    proximity: TerritoryProximity,
    balance: Option<TerritoryBalance>,
) -> (Feature, TerritoryFixtureContexts) {
    let vehicle_d0 = build_vehicle("v_d0", "d0");
    let vehicle_d1 = build_vehicle("v_d1", "d1");

    let fleet =
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_d0).add_vehicle(vehicle_d1).build();

    let actor_d0 = get_test_actor_from_fleet(&fleet, "v_d0");
    let actor_d1 = get_test_actor_from_fleet(&fleet, "v_d1");

    let job_near = TestSingleBuilder::default().id("job_near").location(Some(5)).build_shared();
    let job_far = TestSingleBuilder::default().id("job_far").location(Some(95)).build_shared();

    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(
        Jobs::new(
            &fleet,
            vec![Job::Single(job_near.clone()), Job::Single(job_far.clone())],
            transport.as_ref(),
            &test_logger(),
        )
        .unwrap(),
    );

    let anchors = HashMap::from([("d0".to_string(), 0usize), ("d1".to_string(), 100usize)]);

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(proximity)
        .set_balance(balance)
        .set_anchors(anchors)
        .build()
        .unwrap();

    let correct_assignment = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with(actor_d0.clone(), job_near.clone(), 5),
            route_with(actor_d1.clone(), job_far.clone(), 95),
        ])
        .build();

    let swapped_assignment = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with(actor_d0, job_far, 95), route_with(actor_d1, job_near, 5)])
        .build();

    (feature, TerritoryFixtureContexts { correct_assignment, swapped_assignment })
}

#[test]
fn pull_is_zero_when_every_job_sits_on_its_nearest_anchor() {
    let (feature, ctx) = territory_fixture(TerritoryProximity::Distance, None);
    let objective = feature.objective.unwrap();
    assert_eq!(objective.fitness(&ctx.correct_assignment), 0.0);
}

#[test]
fn pull_penalizes_a_job_served_by_the_far_anchor() {
    let (feature, ctx) = territory_fixture(TerritoryProximity::Distance, None);
    let objective = feature.objective.unwrap();
    assert!(objective.fitness(&ctx.swapped_assignment) > 0.0);
}
