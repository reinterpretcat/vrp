use crate::construction::features::territory::TerritoryFitnessSolutionState;
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
    route_with_jobs(actor, vec![(job, job_location)])
}

/// Builds a route carrying zero or more jobs, in tour order. Used by the balanced-push fixture,
/// where a route may need several jobs (to create a surplus) or none at all (to create a
/// deficit).
fn route_with_jobs(actor: Arc<Actor>, jobs: Vec<(Arc<Single>, usize)>) -> RouteContext {
    let route = Route {
        actor,
        tour: {
            let mut tour = Tour::default();
            tour.set_start(ActivityBuilder::with_location(0).job(None).build());
            tour.set_end(ActivityBuilder::with_location(0).job(None).build());
            for (job, job_location) in jobs {
                tour.insert_last(ActivityBuilder::with_location(job_location).job(Some(job)).build());
            }
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

/// The two solution-level contexts a balanced-push fixture builds, both primed via
/// `accept_solution_state`: `balanced` has each driver's load exactly at quota (`push == 0`);
/// `overloaded` piles every job onto "d0", leaving "d1" idle (`push > 0`).
struct TerritoryBalanceFixtureContexts {
    balanced: InsertionContext,
    overloaded: InsertionContext,
}

/// Builds a territory feature (balanced on the given `balance` metric) plus two primed insertion
/// contexts, over the same two-driver anchors as [`territory_fixture`]: "d0" at location 0, "d1"
/// at location 100, jobs "job_near" (location 5) and "job_far" (location 95). Both drivers share
/// an identical time window, so quotas split the total balance metric 50/50; since the two jobs
/// sit symmetrically around the anchors, every balance metric weighs them equally, so:
/// - `balanced` puts one job per route: each driver's load lands exactly on its quota.
/// - `overloaded` puts both jobs on "d0" and leaves "d1" idle: "d0" carries a surplus and "d1" a
///   deficit.
fn territory_balanced_fixture(balance: TerritoryBalance) -> (Feature, TerritoryBalanceFixtureContexts) {
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

    let mut builder = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(balance))
        .set_anchors(anchors);

    if matches!(balance, TerritoryBalance::ProductionValue) {
        // Exercise the caller-supplied value function (rather than the `1.0` default) to prove
        // the balance metric is actually plumbed through it.
        builder = builder.set_job_value_fn(|_| 4.0);
    }

    let feature = builder.build().unwrap();
    let state = feature.state.as_ref().unwrap();

    let mut balanced = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with(actor_d0.clone(), job_near.clone(), 5),
            route_with(actor_d1.clone(), job_far.clone(), 95),
        ])
        .build();
    state.accept_solution_state(&mut balanced.solution);

    let mut overloaded = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(actor_d0, vec![(job_near, 5), (job_far, 95)]),
            route_with_jobs(actor_d1, vec![]),
        ])
        .build();
    state.accept_solution_state(&mut overloaded.solution);

    (feature, TerritoryBalanceFixtureContexts { balanced, overloaded })
}

#[test]
fn push_is_zero_when_loads_equal_quotas() {
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities);
    let data = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(data.push, 0.0);
}

#[test]
fn push_is_positive_when_imbalanced() {
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities);
    let data = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert!(data.push > 0.0);
}

// Parametrise the imbalanced-push over every balance metric to prove the metric plumbing.
#[test]
fn push_reacts_to_imbalance_for_all_metrics() {
    for balance in [
        TerritoryBalance::Distance,
        TerritoryBalance::Duration,
        TerritoryBalance::Activities,
        TerritoryBalance::ProductionValue,
    ] {
        let (_f, ctx) = territory_balanced_fixture(balance);
        let data = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
        assert!(data.push > 0.0, "push must be positive when imbalanced for {balance:?}");
    }
}
