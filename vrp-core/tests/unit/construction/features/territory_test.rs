use crate::construction::features::territory::TerritoryFitnessSolutionState;
use crate::construction::features::{TerritoryBalance, TerritoryFeatureBuilder, TerritoryProximity};
use crate::construction::heuristics::{InsertionContext, MoveContext, RouteContext, RouteState};
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

/// Regression test for the double-normalization bug: `fitness()` must return the RAW PULL+PUSH
/// magnitude, with normalization exposed ONLY via `fitness_scale()` (per the sibling convention
/// in `vehicle_distance.rs`/`period_balance.rs`/`tour_compactness.rs`). A `WeightedSumScalar`
/// combinator divides `fitness() / fitness_scale()` itself; if `fitness()` already divided by
/// `reference`, that division would apply twice, shrinking territory's contribution to
/// `(pull+push)/reference^2` and effectively disabling the objective.
///
/// Fixture geometry (`territory_fixture`, `swapped_assignment`, balance disabled so `push == 0`):
/// anchors d0@0, d1@100; job_near@5 assigned to d1, job_far@95 assigned to d0.
/// - PULL(job_far on d0) = dist(95, assigned=0) - dist(95, nearest=100) = 95 - 5 = 90
/// - PULL(job_near on d1) = dist(5, assigned=100) - dist(5, nearest=0) = 95 - 5 = 90
/// - raw fitness = pull + push = 90 + 90 + 0 = 180
/// - fitness_scale (`reference`) = sum over all jobs of nearest-anchor proximity
///   = dist(5, nearest=0) + dist(95, nearest=100) = 5 + 5 = 10
#[test]
fn fitness_is_raw_and_fitness_scale_is_the_reference() {
    let (feature, ctx) = territory_fixture(TerritoryProximity::Distance, None);
    let objective = feature.objective.unwrap();

    assert_eq!(objective.fitness(&ctx.swapped_assignment), 180.0);
    assert_eq!(objective.fitness_scale(), 10.0);
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
fn territory_balanced_fixture(balance: TerritoryBalance, allow_idle: bool) -> (Feature, TerritoryBalanceFixtureContexts) {
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
        .set_anchors(anchors)
        .set_allow_idle_drivers(allow_idle);

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
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities, false);
    let data = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(data.push, 0.0);
}

#[test]
fn push_is_positive_when_imbalanced() {
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities, false);
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
        let (_f, ctx) = territory_balanced_fixture(balance, false);
        let data = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
        assert!(data.push > 0.0, "push must be positive when imbalanced for {balance:?}");
    }
}

#[test]
fn allow_idle_drivers_drops_the_idle_driver_from_the_imbalance() {
    // Same overloaded layout as `push_is_positive_when_imbalanced` (every job on "d0", "d1" idle),
    // but with idle drivers allowed: "d1" is excluded from the balance, so the only used driver
    // ("d0") is exactly at its re-based quota -> no surplus -> push == 0. Leaving a driver idle is
    // not an imbalance in this mode.
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities, true);
    let data = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(data.push, 0.0);
}

/// Weighted power cells: a job physically closer (raw distance) to d0's anchor is pulled into
/// d1's cell by a large weight on d1. Serving it on d1 is then penalty-free (it is in its power
/// cell) and serving it on d0 is penalized. Geometry (asserted end-to-end in Task 2 Step 6):
/// job@40 -> raw dist 40 to d0@0, 60 to d1@100; w_d1=30 -> power(d0)=40, power(d1)=30 -> the job
/// belongs to d1's power cell.
#[test]
fn weight_moves_the_boundary_and_zeroes_pull_in_the_power_cell() {
    let vehicle_d0 = build_vehicle("v_d0", "d0");
    let vehicle_d1 = build_vehicle("v_d1", "d1");
    let fleet =
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_d0).add_vehicle(vehicle_d1).build();
    let actor_d0 = get_test_actor_from_fleet(&fleet, "v_d0");
    let actor_d1 = get_test_actor_from_fleet(&fleet, "v_d1");

    // Job at 40: raw dist 40 to d0@0, 60 to d1@100 -> raw-nearest is d0.
    let job = TestSingleBuilder::default().id("job_boundary").location(Some(40)).build_shared();

    let transport = TestTransportCost::new_shared();
    let jobs =
        Arc::new(Jobs::new(&fleet, vec![Job::Single(job.clone())], transport.as_ref(), &test_logger()).unwrap());

    let anchors = HashMap::from([("d0".to_string(), 0usize), ("d1".to_string(), 100usize)]);
    // w_d1 = 30: power(d0) = 40 - 0 = 40, power(d1) = 60 - 30 = 30 -> job belongs to d1's cell.
    let weights = HashMap::from([("d0".to_string(), 0.0), ("d1".to_string(), 30.0)]);

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(None)
        .set_anchors(anchors)
        .set_weights(weights)
        .build()
        .unwrap();

    assert!(feature.objective.is_some());
    let objective = feature.objective.unwrap();

    // On d1 (its power cell): power(d1) - min_power = 30 - 30 = 0.
    let on_d1 =
        TestInsertionContextBuilder::default().with_routes(vec![route_with(actor_d1, job.clone(), 40)]).build();
    assert_eq!(objective.fitness(&on_d1), 0.0);

    // On d0 (foreign cell): power(d0) - min_power = 40 - 30 = 10.
    let on_d0 = TestInsertionContextBuilder::default().with_routes(vec![route_with(actor_d0, job, 40)]).build();
    assert_eq!(objective.fitness(&on_d0), 10.0);
}

/// The nearest-spare leak: with balance on and both drivers exactly at quota, the old pull() found
/// no spare driver and forgave the swapped (cross-boundary) assignment (penalty 0). The
/// power-distance pull() penalizes it: job_far on d0 and job_near on d1 each reach 90 -> 180.
/// (push is 0 because loads equal quotas, so fitness is pure pull.)
#[test]
fn pull_penalizes_swapped_assignment_even_at_quota() {
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
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(TerritoryBalance::Activities))
        .set_anchors(anchors)
        .build()
        .unwrap();
    let state = feature.state.as_ref().unwrap();
    let objective = feature.objective.as_ref().unwrap();

    // Swapped-but-balanced: job_far on d0, job_near on d1. One activity each -> load == quota,
    // so push == 0 and the (old) spare set is empty (old pull() would forgive -> 0).
    let mut swapped = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with(actor_d0, job_far, 95), route_with(actor_d1, job_near, 5)])
        .build();
    state.accept_solution_state(&mut swapped.solution);

    let data = swapped.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(data.push, 0.0);
    // PULL(job_far on d0) = (95 - 0) - min(95, 5) = 90; PULL(job_near on d1) = 90.
    assert_eq!(data.pull, 180.0);
    assert_eq!(objective.fitness(&swapped), 180.0);
}

/// Builds a two-driver territory feature (d0@0, d1@100) balanced on `balance` with the given
/// `tolerance` deadband, over the jobs given as `(id, location)`. Returns the feature, both actors,
/// and the created singles (in the given order) so a test can lay them onto routes.
fn feature_with_jobs_and_tolerance(
    balance: TerritoryBalance,
    tolerance: f64,
    job_specs: &[(&str, usize)],
) -> (Feature, Arc<Actor>, Arc<Actor>, Vec<Arc<Single>>) {
    let vehicle_d0 = build_vehicle("v_d0", "d0");
    let vehicle_d1 = build_vehicle("v_d1", "d1");
    let fleet =
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_d0).add_vehicle(vehicle_d1).build();
    let actor_d0 = get_test_actor_from_fleet(&fleet, "v_d0");
    let actor_d1 = get_test_actor_from_fleet(&fleet, "v_d1");

    let singles: Vec<Arc<Single>> = job_specs
        .iter()
        .map(|(id, loc)| TestSingleBuilder::default().id(id).location(Some(*loc)).build_shared())
        .collect();

    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(
        Jobs::new(&fleet, singles.iter().cloned().map(Job::Single).collect(), transport.as_ref(), &test_logger())
            .unwrap(),
    );

    let anchors = HashMap::from([("d0".to_string(), 0usize), ("d1".to_string(), 100usize)]);
    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(balance))
        .set_balance_tolerance(tolerance)
        .set_anchors(anchors)
        .build()
        .unwrap();

    (feature, actor_d0, actor_d1, singles)
}

/// FIX 1 (deadband): d0 carries 2 of 3 activities, so its load (2) sits above the exact quota
/// (1.5). With zero tolerance that is billed as PUSH; a 50% deadband widens the quota to 2.25 and
/// forgives the small imbalance entirely.
#[test]
fn balance_tolerance_forgives_small_imbalance() {
    let specs = [("j0", 1), ("j1", 2), ("j2", 98)];

    let (feat0, a0, a1, s) = feature_with_jobs_and_tolerance(TerritoryBalance::Activities, 0.0, &specs);
    let mut strict = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(a0, vec![(s[0].clone(), 1), (s[1].clone(), 2)]),
            route_with_jobs(a1, vec![(s[2].clone(), 98)]),
        ])
        .build();
    feat0.state.as_ref().unwrap().accept_solution_state(&mut strict.solution);
    let push_strict = strict.solution.state.get_territory_fitness().cloned().unwrap_or_default().push;
    assert!(push_strict > 0.0, "zero-tolerance bills the small imbalance");

    let (feat1, b0, b1, s2) = feature_with_jobs_and_tolerance(TerritoryBalance::Activities, 0.5, &specs);
    let mut lenient = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(b0, vec![(s2[0].clone(), 1), (s2[1].clone(), 2)]),
            route_with_jobs(b1, vec![(s2[2].clone(), 98)]),
        ])
        .build();
    feat1.state.as_ref().unwrap().accept_solution_state(&mut lenient.solution);
    let push_lenient = lenient.solution.state.get_territory_fitness().cloned().unwrap_or_default().push;
    assert_eq!(push_lenient, 0.0, "the deadband forgives the small imbalance");
}

/// FIX 2 (location-aware PUSH marginal): an over-quota driver's per-insertion shedding pressure
/// must fall on its boundary jobs, not the ones buried deep in its cell. d0 is over quota (carries
/// three jobs against a quota of 2.5); all candidates sit in d0's cell (PULL 0), so the estimate is
/// pure PUSH marginal. The deepest job carries none, a boundary job carries the most.
#[test]
fn push_marginal_sheds_boundary_jobs_not_deep_ones() {
    // Per-job power gaps (dist to d1 anchor − dist to d0 anchor): f1=98, f2=96, f3=94, deep=90,
    // bound=10 -> median (push_reach) = 94. So max(0, 94 − gap): f1 -> 0, deep -> 4, bound -> 84.
    let specs = [("f1", 1), ("f2", 2), ("f3", 3), ("deep", 5), ("bound", 45)];
    let (feature, a0, _a1, s) = feature_with_jobs_and_tolerance(TerritoryBalance::Activities, 0.0, &specs);
    let objective = feature.objective.as_ref().unwrap();

    // d0 carries the three fillers -> load 3 > quota 2.5 -> over quota.
    let mut route_ctx = route_with_jobs(a0, vec![(s[0].clone(), 1), (s[1].clone(), 2), (s[2].clone(), 3)]);
    feature.state.as_ref().unwrap().accept_route_state(&mut route_ctx);

    let ictx = TestInsertionContextBuilder::default().build();
    let estimate = |job: &Arc<Single>| {
        objective.estimate(&MoveContext::route(&ictx.solution, &route_ctx, &Job::Single(job.clone())))
    };

    let deepest = estimate(&s[0]); // gap 98 >= reach 94
    let deep = estimate(&s[3]); // gap 90
    let boundary = estimate(&s[4]); // gap 10

    assert_eq!(deepest, 0.0, "the deepest job carries no shedding pressure — it stays home");
    assert!(boundary > deep, "a boundary job is shed before a deeper one");
    assert!(deep > 0.0, "a mid-depth job still carries some pressure");
}

/// The deadband also gates the per-insertion PUSH marginal: with the driver inside the (widened)
/// band it is not over quota, so even a boundary job carries no shedding pressure.
#[test]
fn push_marginal_is_zero_within_the_deadband() {
    let specs = [("f1", 1), ("f2", 2), ("f3", 3), ("bound", 45)];
    // Quota = 4 activities / 2 = 2.0; a 100% deadband widens it to 4.0, so a load-3 route is inside.
    let (feature, a0, _a1, s) = feature_with_jobs_and_tolerance(TerritoryBalance::Activities, 1.0, &specs);
    let objective = feature.objective.as_ref().unwrap();

    let mut route_ctx = route_with_jobs(a0, vec![(s[0].clone(), 1), (s[1].clone(), 2), (s[2].clone(), 3)]);
    feature.state.as_ref().unwrap().accept_route_state(&mut route_ctx);

    let ictx = TestInsertionContextBuilder::default().build();
    let boundary = objective.estimate(&MoveContext::route(&ictx.solution, &route_ctx, &Job::Single(s[3].clone())));
    assert_eq!(boundary, 0.0, "no shedding pressure while the driver is inside the deadband");
}

/// A job whose only compatible driver is the geographically-far one incurs ZERO overlap penalty
/// when served by that driver: `nearest_power`'s reference ranges over compatible anchors only, so
/// the far (only) compatible seed IS the reference. Proves a skill/constraint-forced
/// cross-territory assignment is not penalized (feasibility is handled by MinimizeUnassigned above).
#[test]
fn skill_forced_far_assignment_is_not_penalized() {
    let vehicle_d0 = build_vehicle("v_d0", "d0");
    let vehicle_d1 = build_vehicle("v_d1", "d1");
    let fleet =
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_d0).add_vehicle(vehicle_d1).build();
    let actor_d0 = get_test_actor_from_fleet(&fleet, "v_d0");
    let actor_d1 = get_test_actor_from_fleet(&fleet, "v_d1");

    // Job at 10: raw-nearest anchor is d0@0 (dist 10) vs d1@100 (dist 90). But only d1 is compatible.
    let job = TestSingleBuilder::default().id("job_skill").location(Some(10)).build_shared();

    let transport = TestTransportCost::new_shared();
    let jobs =
        Arc::new(Jobs::new(&fleet, vec![Job::Single(job.clone())], transport.as_ref(), &test_logger()).unwrap());

    let anchors = HashMap::from([("d0".to_string(), 0usize), ("d1".to_string(), 100usize)]);

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        // Only d1 may serve the job (stand-in for a skill / day-availability restriction).
        .set_compatibility_fn(|_, actor| actor.vehicle.dimens.get_driver_id().map(|s| s == "d1").unwrap_or(false))
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(None)
        .set_anchors(anchors)
        .build()
        .unwrap();
    let objective = feature.objective.unwrap();

    // Served by d1 (its only compatible driver): reference = min over compatible = power(d1) = 90,
    // assigned = power(d1) = 90 -> penalty 0, even though the job is geographically near d0.
    let on_d1 = TestInsertionContextBuilder::default().with_routes(vec![route_with(actor_d1, job, 10)]).build();
    assert_eq!(objective.fitness(&on_d1), 0.0);
    let _ = actor_d0;
}
