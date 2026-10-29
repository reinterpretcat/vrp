use crate::construction::enablers::{PaidWorkingDurationTourState, TotalDistanceTourState, TotalDurationTourState};
use crate::construction::features::territory::{
    TerritoryFitnessSolutionState, TerritoryRouteQuotaTourState, TerritoryShared,
};
use crate::construction::features::{
    TerritoryBalance, TerritoryFeatureBuilder, TerritoryFitnessData, TerritoryProximity,
};
use crate::construction::heuristics::{ActivityContext, InsertionContext, MoveContext, RouteContext, RouteState};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::domain::test_logger;
use crate::helpers::models::problem::{
    FleetBuilder, TestSingleBuilder, TestTransportCost, TestVehicleBuilder, fake_routing, get_test_actor_from_fleet,
    test_driver,
};
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::Feature;
use crate::models::common::TimeInterval;
use crate::models::problem::{Actor, DriverIdDimension, Job, Jobs, Single, VehicleDetail, VehiclePlace};
use crate::models::solution::{Route, Tour};
use crate::prelude::Float;
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
    build_vehicle_with_shifts(id, driver_id, 1)
}

/// [`build_vehicle`] over `shifts` identical `[0, 1000]` shift windows. Each detail becomes its own
/// `Actor` (see `Fleet::new`), so a driver built with two shifts is two actors — hence two routes —
/// carrying one shared, horizon-wide quota between them.
fn build_vehicle_with_shifts(id: &str, driver_id: &str, shifts: usize) -> crate::models::problem::Vehicle {
    let mut builder = TestVehicleBuilder::default();
    builder.id(id).details(
        (0..shifts)
            .map(|_| VehicleDetail {
                start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
                end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
            })
            .collect(),
    );
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
    let locations: Vec<usize> = jobs.iter().map(|(_, location)| *location).collect();

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

    let mut route_ctx = RouteContext::new_with_state(route, RouteState::default());

    // These routes are assembled by hand and never see `update_statistics`, so the totals the
    // transport feature would have cached are written here instead. `Distance` and `Duration`
    // balance on exactly those, and a fixture that left them unset would read as a route that
    // travelled nowhere — which passes for the wrong reason rather than failing.
    //
    // The model matches `TestTransportCost` (distance == duration == |from - to|) over the closed
    // tour 0 -> job -> ... -> job -> 0.
    let total = closed_tour_travel(&locations);
    route_ctx.state_mut().set_total_distance(total);
    route_ctx.state_mut().set_total_duration(total);
    // These activities carry no service time, so over the default depot-to-depot span the paid
    // working duration is the same travel the other two totals are.
    route_ctx.state_mut().set_paid_working_duration(total);

    route_ctx
}

/// Whole-route travel over `0 -> locations... -> 0` under `TestTransportCost`.
fn closed_tour_travel(locations: &[usize]) -> Float {
    if locations.is_empty() {
        return 0.0;
    }

    let mut previous = 0usize;
    let mut total = 0.0;

    for location in locations {
        total += fake_routing(previous, *location);
        previous = *location;
    }

    total + fake_routing(previous, 0)
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
    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);
    territory_fixture_with_anchors(proximity, balance, anchors)
}

/// [`territory_fixture`] over an arbitrary anchor map, so a test can vary the one input the solver
/// no longer derives.
fn territory_fixture_with_anchors(
    proximity: TerritoryProximity,
    balance: Option<TerritoryBalance>,
    anchors: HashMap<String, Vec<usize>>,
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

/// An EMPTY anchor map leaves the objective inert, it does not fall back to anything. The solver
/// used to derive anchors from the jobs when the caller sent none, so this same fixture scored the
/// swapped assignment as a penalty either way; now nothing is derived and no driver holds ground,
/// so the assignment that is maximally wrong under real anchors costs exactly zero. Distinguishing
/// "inert" from "derived" is the whole point — the assertion is `== 0.0`, not `is_ok()`, because
/// only a zero proves nothing was invented in place of the missing input.
#[test]
fn an_empty_anchor_map_leaves_the_objective_inert() {
    let (feature, ctx) = territory_fixture_with_anchors(TerritoryProximity::Distance, None, HashMap::new());
    let objective = feature.objective.unwrap();

    assert_eq!(objective.fitness(&ctx.swapped_assignment), 0.0);
    assert_eq!(objective.fitness(&ctx.correct_assignment), 0.0);
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
fn territory_balanced_fixture(
    balance: TerritoryBalance,
    allow_idle: bool,
) -> (Feature, TerritoryBalanceFixtureContexts) {
    territory_balanced_fixture_with_quotas(balance, allow_idle, HashMap::new())
}

/// [`territory_balanced_fixture`] with a caller-supplied per-driver quota map. An empty map is the
/// "no quota supplied" case and must reproduce the derived path exactly.
fn territory_balanced_fixture_with_quotas(
    balance: TerritoryBalance,
    allow_idle: bool,
    quotas: HashMap<String, Float>,
) -> (Feature, TerritoryBalanceFixtureContexts) {
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

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);

    let mut builder = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(balance))
        .set_anchors(anchors)
        .set_quotas(quotas)
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

/// Pins the exact numbers the DERIVED quota path produces, so a change to how quotas are resolved
/// cannot quietly move them. Two equal-window drivers and two activities ⇒ derived quota 1.0 each:
/// - `balanced` (one job per driver): both loads sit on quota ⇒ no surplus ⇒ PUSH 0; each job sits
///   on its own nearest anchor ⇒ PULL 0.
/// - `overloaded` (both jobs on "d0", "d1" idle): surplus 1 against a quota of 1, billed at the
///   only deficit anchor's π(0, 100) = 100. PUSH is convex in surplus —
///   `surplus / avg_metric × gain × surplus / quota × π` — and this fixture sits at surplus ==
///   quota, so the convexity factor is the whole [`PUSH_CONVEXITY_GAIN`] of 3. The metric is
///   activity count, so `avg_metric` is 1 and the leading normalization is the identity here ⇒
///   PUSH 1 × 3 × 100 = 300; job_far@95 served from anchor 0 reaches 95 − 5 = 90 ⇒ PULL 90.
#[test]
fn derived_quotas_produce_exact_pull_and_push() {
    let (_f, ctx) = territory_balanced_fixture(TerritoryBalance::Activities, false);

    let balanced = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    let overloaded = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();

    assert_eq!(balanced.pull, 0.0);
    assert_eq!(balanced.push, 0.0);
    assert_eq!(overloaded.pull, 90.0);
    assert_eq!(overloaded.push, 300.0);
}

/// Reads the caller's quota verbatim rather than merging it with, or correcting it against, the
/// derived one. The derived quota here is 1.0 per driver (pinned above), so supplying
/// `{d0: 2, d1: 0}` inverts which layout is balanced:
/// - `balanced` (one job each): "d1" is now 1.0 over its zero quota while "d0" is a deficit, so the
///   surplus ships at π(100, 0) = 100 ⇒ PUSH 100, where the derived quota gave 0. A quota of exactly
///   zero has no scale to normalize the convex term against and is already maximal imbalance, so
///   PUSH stays linear there and the number is the same one the linear formula gave.
/// - `overloaded` (both jobs on "d0"): "d0" sits exactly on its quota of 2 and idle "d1" is not
///   below its quota of 0, so nothing is surplus ⇒ PUSH 0, where the derived quota gave 100.
#[test]
fn supplied_quotas_are_used_verbatim() {
    let quotas = HashMap::from([("d0".to_string(), 2.0), ("d1".to_string(), 0.0)]);
    let (_f, ctx) = territory_balanced_fixture_with_quotas(TerritoryBalance::Activities, false, quotas);

    let balanced = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    let overloaded = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();

    assert_eq!(balanced.push, 100.0);
    assert_eq!(overloaded.push, 0.0);
}

/// An empty supplied map is exactly "no quota supplied": the derived path runs untouched, so the
/// numbers pinned by `derived_quotas_produce_exact_pull_and_push` come back byte for byte.
#[test]
fn empty_supplied_quotas_reproduce_the_derivation() {
    let (_f, ctx) = territory_balanced_fixture_with_quotas(TerritoryBalance::Activities, false, HashMap::new());

    let balanced = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    let overloaded = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();

    assert_eq!(balanced.pull, 0.0);
    assert_eq!(balanced.push, 0.0);
    assert_eq!(overloaded.pull, 90.0);
    assert_eq!(overloaded.push, 300.0);
}

/// A key matching no driver in the fleet is dropped, never fatal: it cannot be keyed to any route,
/// so it can only be noise. The outcome is identical to `supplied_quotas_are_used_verbatim`, which
/// also proves the unknown key did not displace or perturb the two that do match.
#[test]
fn unknown_driver_key_in_supplied_quotas_is_ignored() {
    let quotas = HashMap::from([("d0".to_string(), 2.0), ("d1".to_string(), 0.0), ("nobody".to_string(), 999.0)]);
    let (_f, ctx) = territory_balanced_fixture_with_quotas(TerritoryBalance::Activities, false, quotas);

    let balanced = ctx.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    let overloaded = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();

    assert_eq!(balanced.push, 100.0);
    assert_eq!(overloaded.push, 0.0);
}

/// A fleet driver the caller omitted is left OUT of the balance: never a surplus, never a deficit.
/// Two fixtures, because each rules out one of the two alternative readings.
#[test]
fn driver_absent_from_supplied_quotas_is_left_out_of_the_balance() {
    // Only "d0" is quoted, generously (5 against a load of 1), so "d0" can never be the surplus.
    // What "d1" is decides the PUSH on the balanced layout:
    // - left out (the chosen behaviour): "d1" is neither surplus nor deficit ⇒ 0.
    // - read as "quota 0": "d1"'s single job would be a surplus shipped to "d0" at π(100, 0) ⇒ 100.
    let (_f, generous) = territory_balanced_fixture_with_quotas(
        TerritoryBalance::Activities,
        false,
        HashMap::from([("d0".to_string(), 5.0)]),
    );
    let balanced = generous.balanced.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(balanced.push, 0.0, "an omitted driver must not be read as quota 0");

    // Only "d0" is quoted, tightly (0.5 against a load of 2), so "d0" IS the surplus on the
    // overloaded layout. What "d1" is decides again:
    // - left out (the chosen behaviour): there is no deficit to ship the surplus to ⇒ 0.
    // - back-filled with the DERIVED quota of 1.0: idle "d1" would be a deficit and the 1.5 surplus
    //   would ship at π(0, 100) ⇒ 150 — which would reintroduce exactly the total-demand quota the
    //   supplied map exists to replace.
    let (_f, tight) = territory_balanced_fixture_with_quotas(
        TerritoryBalance::Activities,
        false,
        HashMap::from([("d0".to_string(), 0.5)]),
    );
    let overloaded = tight.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(overloaded.push, 0.0, "an omitted driver must not be back-filled from the derivation");
}

/// Supplied quotas bypass the `allow_idle_drivers` re-basing, because they are already the intended
/// targets: re-basing would overwrite the caller's numbers with a capacity share it deliberately did
/// not ask for. Same overloaded layout as
/// `allow_idle_drivers_drops_the_idle_driver_from_the_imbalance` (both jobs on "d0", "d1" idle),
/// which the DERIVED path re-bases onto "d0" alone (quota 2) for PUSH 0. With an explicit 1/1 quota
/// there is no re-basing: "d0" is 1 over its quota of 1 and idle "d1" is a deficit ⇒ PUSH
/// 1 × 3 × 100 = 300 (surplus == quota, so the convex factor is the full gain).
#[test]
fn supplied_quotas_are_not_rebased_when_idle_drivers_are_allowed() {
    let quotas = HashMap::from([("d0".to_string(), 1.0), ("d1".to_string(), 1.0)]);
    let (_f, ctx) = territory_balanced_fixture_with_quotas(TerritoryBalance::Activities, true, quotas);

    let data = ctx.overloaded.solution.state.get_territory_fitness().cloned().unwrap_or_default();
    assert_eq!(data.push, 300.0);
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
    let jobs = Arc::new(Jobs::new(&fleet, vec![Job::Single(job.clone())], transport.as_ref(), &test_logger()).unwrap());

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);
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
    let on_d1 = TestInsertionContextBuilder::default().with_routes(vec![route_with(actor_d1, job.clone(), 40)]).build();
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

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);

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

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);
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
    //
    // Driven through `accept_solution_state`, not `accept_route_state`: the route's slice of the
    // quota depends on which routes its driver actually has, so it is the solution pass that
    // computes it.
    let mut ictx = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with_jobs(a0, vec![(s[0].clone(), 1), (s[1].clone(), 2), (s[2].clone(), 3)])])
        .build();
    feature.state.as_ref().unwrap().accept_solution_state(&mut ictx.solution);
    // The marginal is priced at the ACTIVITY now, where the detour is known, so the estimate has
    // to be asked for there. `prev` and `next` are the route's own first stop, so the detour is
    // the round trip out to the candidate and back — the same ordering the old route-level reading
    // produced, at a position the search actually considers.
    let anchor_stop = ActivityBuilder::with_location(1).job(None).build();
    let estimate = |job: &Arc<Single>, location: usize| {
        let target = ActivityBuilder::with_location(location).job(Some(job.clone())).build();
        let activity_ctx = ActivityContext { index: 1, prev: &anchor_stop, target: &target, next: Some(&anchor_stop) };
        objective.estimate(&MoveContext::activity(&ictx.solution, &ictx.solution.routes[0], &activity_ctx))
    };

    let deepest = estimate(&s[0], 1); // gap 98 >= reach 94
    let deep = estimate(&s[3], 5); // gap 90
    let boundary = estimate(&s[4], 45); // gap 10

    assert_eq!(deepest, 0.0, "the deepest job carries no shedding pressure — it stays home");
    assert!(boundary > deep, "a boundary job is shed before a deeper one");
    assert!(deep > 0.0, "a mid-depth job still carries some pressure");
}

/// A driver with TWO shifts is two actors, hence two routes, while its quota spans the whole
/// horizon. `push_marginal` compared ONE route's load against that horizon-wide quota, so a single
/// day's load could never exceed a whole horizon's worth and the per-insertion shedding pressure was
/// inert in every multi-day problem. No other fixture reaches this: they all give each driver a
/// single shift, where the route's share IS the whole quota and the two readings coincide.
///
/// "d0" holds two shifts (capacity 2000) against "d1"'s one (1000), so of the six activities "d0"
/// draws a horizon quota of `6 × 2000/3000 = 4` and "d1" one of 2. One of "d0"'s two routes carries
/// three of the jobs:
/// - against the horizon quota of 4, a load of 3 is inside the band ⇒ the old comparison returned
///   0.0, asserted below as the control so this cannot pass by accident;
/// - against this route's share, `4 × 1000/2000 = 2`, the load is 1 over ⇒ pressure fires.
///
/// The value: the marginal is the derivative of the convex fitness, `2 × GAIN × surplus / quota`,
/// so with `surplus_ratio = (3 − 2)/2 = 0.5` and a gain of 3 the price factor is `2 × 3 × 0.5 = 3`.
/// The probe job "bound"@45 has power gap `55 − 45 = 10` against a `push_reach` of 94 (the median of
/// the six gaps 98, 96, 94, 92, 90, 10), and `value_factor` is 1 (Activities, so metric and average
/// are both 1) ⇒ `1 × 3 × 84 = 252`. PULL is 0 for that job — it sits in d0's own cell — so the
/// estimate is the marginal alone.
#[test]
fn push_marginal_fires_when_one_shift_of_several_is_over_its_share() {
    let vehicle_d0 = build_vehicle_with_shifts("v_d0", "d0", 2);
    let vehicle_d1 = build_vehicle_with_shifts("v_d1", "d1", 1);
    let fleet =
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_d0).add_vehicle(vehicle_d1).build();

    let actors_of = |driver: &str| -> Vec<Arc<Actor>> {
        fleet
            .actors
            .iter()
            .filter(|actor| actor.vehicle.dimens.get_driver_id().map(String::as_str) == Some(driver))
            .cloned()
            .collect()
    };
    let d0_actors = actors_of("d0");
    let d1_actors = actors_of("d1");
    assert_eq!((d0_actors.len(), d1_actors.len()), (2, 1), "two shifts must yield two actors");

    let specs = [("f1", 1), ("f2", 2), ("f3", 3), ("f4", 4), ("deep", 5), ("bound", 45)];
    let singles: Vec<Arc<Single>> =
        specs.iter().map(|(id, loc)| TestSingleBuilder::default().id(id).location(Some(*loc)).build_shared()).collect();

    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(
        Jobs::new(&fleet, singles.iter().cloned().map(Job::Single).collect(), transport.as_ref(), &test_logger())
            .unwrap(),
    );

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);
    let all_actors: Vec<Arc<Actor>> = d0_actors.iter().chain(d1_actors.iter()).cloned().collect();

    // Built directly, because the horizon quota and the capacities are the control this test turns
    // on and neither is reachable through `Feature`.
    let shared = TerritoryShared::new(
        transport.clone(),
        all_actors.clone(),
        jobs.clone(),
        Arc::new(|_: &Job, _: &Actor| true),
        TerritoryProximity::Distance,
        Some(TerritoryBalance::Activities),
        0.0,
        // deficit_weight: zero keeps PUSH one-sided, which is what every assertion below was
        // measured against.
        0.0,
        anchors.clone(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Arc::new(|_: &Job| 1.0),
        false,
    );
    assert_eq!(shared.caps.get("d0").copied(), Some(2000.0), "two shifts of 1000 each");
    assert_eq!(shared.caps.get("d1").copied(), Some(1000.0));
    assert_eq!(shared.quotas.get("d0").copied(), Some(4.0), "the quota spans the driver's whole horizon");

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(all_actors)
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(TerritoryBalance::Activities))
        .set_anchors(anchors)
        .build()
        .unwrap();
    let objective = feature.objective.as_ref().unwrap();

    // d0 works BOTH of its shifts, and one of the two routes carries three of the six jobs while
    // the other carries none. The driver is inside its horizon quota; the day is not.
    let mut ictx = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(
                d0_actors[0].clone(),
                vec![(singles[0].clone(), 1), (singles[1].clone(), 2), (singles[2].clone(), 3)],
            ),
            route_with_jobs(d0_actors[1].clone(), vec![]),
        ])
        .build();
    feature.state.as_ref().unwrap().accept_solution_state(&mut ictx.solution);
    // The control: measured against the horizon quota the route is INSIDE the band, which is exactly
    // why the old reading returned zero here.
    assert!(3.0 <= shared.over_quota(4.0), "the horizon-wide comparison must be inert on this fixture");

    let anchor_stop = ActivityBuilder::with_location(45).job(None).build();
    let target = ActivityBuilder::with_location(45).job(Some(singles[5].clone())).build();
    let activity_ctx = ActivityContext { index: 1, prev: &anchor_stop, target: &target, next: Some(&anchor_stop) };
    let estimate = objective.estimate(&MoveContext::activity(&ictx.solution, &ictx.solution.routes[0], &activity_ctx));

    assert!(estimate > 0.0, "the route is over ITS share of the quota, so the marginal must fire");
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
    let jobs = Arc::new(Jobs::new(&fleet, vec![Job::Single(job.clone())], transport.as_ref(), &test_logger()).unwrap());

    let anchors = HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]);

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

// region: per-driver anchor LISTS

/// Builds a territory feature over the two standard drivers ("d0", "d1") with caller-given anchor
/// LISTS and the jobs given as `(id, location)`. Returns the feature, both actors, and the created
/// singles in the given order so a test can lay them onto routes.
fn feature_with_anchor_lists(
    anchors: HashMap<String, Vec<usize>>,
    balance: Option<TerritoryBalance>,
    job_specs: &[(&str, usize)],
) -> (Feature, Arc<Actor>, Arc<Actor>, Vec<Arc<Single>>) {
    feature_with_anchor_lists_and_day_patch(anchors, balance, job_specs, false)
}

/// [`feature_with_anchor_lists`] with the `day_patch` option set as given.
fn feature_with_anchor_lists_and_day_patch(
    anchors: HashMap<String, Vec<usize>>,
    balance: Option<TerritoryBalance>,
    job_specs: &[(&str, usize)],
    day_patch: bool,
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

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(balance)
        .set_anchors(anchors)
        .set_day_patch(day_patch)
        .build()
        .unwrap();

    (feature, actor_d0, actor_d1, singles)
}

/// The primed PULL/PUSH for the standard two-job scenario (job_near@5, job_far@95) laid out with
/// BOTH jobs on "d0" and "d1" left idle, under the given anchor lists and balance metric.
fn both_jobs_on_d0_fitness(
    anchors: HashMap<String, Vec<usize>>,
    balance: Option<TerritoryBalance>,
) -> TerritoryFitnessData {
    let (feature, actor_d0, actor_d1, singles) =
        feature_with_anchor_lists(anchors, balance, &[("job_near", 5), ("job_far", 95)]);

    let mut ctx = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(actor_d0, vec![(singles[0].clone(), 5), (singles[1].clone(), 95)]),
            route_with_jobs(actor_d1, vec![]),
        ])
        .build();
    feature.state.as_ref().unwrap().accept_solution_state(&mut ctx.solution);

    ctx.solution.state.get_territory_fitness().cloned().unwrap_or_default()
}

/// A driver holding several anchors is judged by its NEAREST one. "d0" holds anchors at 0 and 100
/// while "d1" holds a single one at 50, and "d0" serves a job beside each of its own anchors: both
/// sit 5 from the nearer of the pair (against 45 to "d1"), so neither reaches into a foreign cell
/// and PULL is 0. Collapsing "d0" to its first anchor alone would leave job_far 95 from its driver
/// against a reference of 45, i.e. PULL 50 — asserted as the control, so this cannot pass by
/// accident.
#[test]
fn a_driver_is_judged_by_its_nearest_anchor_of_several() {
    let specs = [("job_near", 5), ("job_far", 95)];

    let (paired, a0, _a1, s) = feature_with_anchor_lists(
        HashMap::from([("d0".to_string(), vec![0, 100]), ("d1".to_string(), vec![50])]),
        None,
        &specs,
    );
    let on_paired = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with_jobs(a0, vec![(s[0].clone(), 5), (s[1].clone(), 95)])])
        .build();
    assert_eq!(paired.objective.unwrap().fitness(&on_paired), 0.0, "each job sits beside one of d0's own anchors");

    let (single, b0, _b1, s2) = feature_with_anchor_lists(
        HashMap::from([("d0".to_string(), vec![0]), ("d1".to_string(), vec![50])]),
        None,
        &specs,
    );
    let on_single = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with_jobs(b0, vec![(s2[0].clone(), 5), (s2[1].clone(), 95)])])
        .build();
    assert_eq!(single.objective.unwrap().fitness(&on_single), 50.0, "with only the first anchor, job_far is foreign");
}

/// An empty anchor list is exactly an absent key: that driver takes no part in the territory. It is
/// not a job's overlap reference — a bug that read "no anchors" as proximity 0 would make it the
/// nearest reference for every job and inflate the OTHER drivers' PULL — and it is neither a PUSH
/// source nor a PUSH target.
#[test]
fn an_empty_anchor_list_behaves_exactly_like_an_absent_driver() {
    let empty = both_jobs_on_d0_fitness(
        HashMap::from([("d0".to_string(), vec![0]), ("d1".to_string(), Vec::new())]),
        Some(TerritoryBalance::Activities),
    );
    let absent =
        both_jobs_on_d0_fitness(HashMap::from([("d0".to_string(), vec![0])]), Some(TerritoryBalance::Activities));
    let anchored = both_jobs_on_d0_fitness(
        HashMap::from([("d0".to_string(), vec![0]), ("d1".to_string(), vec![100])]),
        Some(TerritoryBalance::Activities),
    );

    assert_eq!((empty.pull, empty.push), (absent.pull, absent.push), "empty list and absent key must agree");
    // "d1" is no reference, so both jobs sit in d0's only cell (PULL 0); and "d1" is no deficit, so
    // d0's surplus has nowhere to ship (PUSH 0).
    assert_eq!((empty.pull, empty.push), (0.0, 0.0));
    // The same layout with "d1" actually anchored: job_far reaches 90 past d1's cell, and d0's
    // surplus of one activity against a quota of 1 is billed at π(0, 100) = 100, times the full
    // convexity gain of 3 (surplus == quota) ⇒ 300.
    assert_eq!((anchored.pull, anchored.push), (90.0, 300.0));
}

/// The PUSH ground cost between two drivers is the minimum over their ANCHOR PAIRS, not the distance
/// between one designated anchor each. Over-quota "d0" holds anchors at 0 and 90; the only deficit
/// driver "d1" holds one at 100. The nearest pair is (90, 100) = 10, so the surplus of one activity
/// is billed at 10 — against 100 when "d0" holds only its first anchor, asserted as the control.
/// Both are then multiplied by the full convexity gain of 3, since surplus == quota here.
#[test]
fn push_ground_cost_is_the_minimum_over_anchor_pairs() {
    let near_pair = both_jobs_on_d0_fitness(
        HashMap::from([("d0".to_string(), vec![0, 90]), ("d1".to_string(), vec![100])]),
        Some(TerritoryBalance::Activities),
    );
    assert_eq!(near_pair.push, 30.0);

    let far_only = both_jobs_on_d0_fitness(
        HashMap::from([("d0".to_string(), vec![0]), ("d1".to_string(), vec![100])]),
        Some(TerritoryBalance::Activities),
    );
    assert_eq!(far_only.push, 300.0);
}

// endregion

// region: day patch

/// "d0" holds two patches far apart (anchors 0 and 100), "d1" one between them (50).
fn day_patch_anchors() -> HashMap<String, Vec<usize>> {
    HashMap::from([("d0".to_string(), vec![0, 100]), ("d1".to_string(), vec![50])])
}

/// The PULL of one "d0" route over the given jobs, with and without `day_patch`.
fn d0_route_pull(job_specs: &[(&str, usize)], day_patch: bool) -> Float {
    let (feature, actor_d0, _actor_d1, singles) =
        feature_with_anchor_lists_and_day_patch(day_patch_anchors(), None, job_specs, day_patch);
    let jobs = singles.iter().cloned().zip(job_specs.iter().map(|(_, loc)| *loc)).collect();
    let ctx = TestInsertionContextBuilder::default().with_routes(vec![route_with_jobs(actor_d0, jobs)]).build();

    feature.objective.unwrap().fitness(&ctx)
}

/// A day that mixes both of "d0"'s patches. Without `day_patch` every job counts against its
/// nearest anchor, so none reaches into a foreign cell and PULL is 0. With it, the route is
/// measured against its one anchor, the one nearest its jobs as a whole: 0 (summed 5 + 10 + 95 =
/// 110 against 95 + 90 + 5 = 190 for 100). job_c then sits 95 from it against a reference of 5,
/// i.e. PULL 90.
#[test]
fn day_patch_penalizes_a_day_that_mixes_two_patches() {
    let specs = [("job_a", 5), ("job_b", 10), ("job_c", 95)];

    assert_eq!(d0_route_pull(&specs, false), 0.0, "each job counts against its nearest anchor");
    assert_eq!(d0_route_pull(&specs, true), 90.0, "job_c is measured against the route's anchor at 0");
}

/// A day whose jobs all sit in one patch costs the same either way. job_d at 40 is 40 from "d0"'s
/// anchor at 0 against 10 to "d1", so PULL is 30 with and without `day_patch` — non-zero, so the
/// equality is not a pair of zeros.
#[test]
fn day_patch_leaves_a_single_patch_day_unchanged() {
    let specs = [("job_a", 5), ("job_d", 40)];

    assert_eq!(d0_route_pull(&specs, false), 30.0);
    assert_eq!(d0_route_pull(&specs, true), 30.0);
}

/// The insertion estimate follows the same rule: once a "d0" route sits in the patch at 0, adding
/// job_c (beside the anchor at 100) costs nothing without `day_patch` and 95 - 5 = 90 with it.
#[test]
fn day_patch_estimate_prices_a_job_from_the_other_patch() {
    let specs = [("job_a", 5), ("job_b", 10), ("job_c", 95)];

    let estimate = |day_patch: bool| {
        let (feature, actor_d0, _actor_d1, s) =
            feature_with_anchor_lists_and_day_patch(day_patch_anchors(), None, &specs, day_patch);
        let mut route_ctx = route_with_jobs(actor_d0, vec![(s[0].clone(), 5), (s[1].clone(), 10)]);
        feature.state.as_ref().unwrap().accept_route_state(&mut route_ctx);
        let ictx = TestInsertionContextBuilder::default().build();

        feature.objective.unwrap().estimate(&MoveContext::route(&ictx.solution, &route_ctx, &Job::Single(s[2].clone())))
    };

    assert_eq!(estimate(false), 0.0);
    assert_eq!(estimate(true), 90.0);
}

// region: independence from hash seeding

/// A `TerritoryShared` and the pieces needed to drive it: the jobs it was built over and the
/// actors its routes must be keyed to.
struct SharedFixture {
    shared: TerritoryShared,
    singles: Vec<Arc<Single>>,
    actors: Vec<Arc<Actor>>,
}

/// Builds the feature's internal `TerritoryShared` directly: these tests need the anchor ranking,
/// the derived quota and the raw PUSH total, and none of the three is reachable through `Feature`.
/// `drivers` is `(driver key, anchor location, shift end)` — the shift end is the driver's
/// capacity, which sizes its derived quota — and every job is compatible with every driver.
fn shared_over(
    drivers: &[(&str, usize, Float)],
    job_locations: &[usize],
    balance: Option<TerritoryBalance>,
    supplied_quotas: HashMap<String, Float>,
) -> SharedFixture {
    shared_over_with_shares(drivers, job_locations, balance, supplied_quotas, HashMap::new(), HashMap::new(), 0.0)
}

/// [`shared_over`] with caller-supplied quota shares and pools — the route-level quota mode.
fn shared_over_with_shares(
    drivers: &[(&str, usize, Float)],
    job_locations: &[usize],
    balance: Option<TerritoryBalance>,
    supplied_quotas: HashMap<String, Float>,
    supplied_shares: HashMap<String, Float>,
    quota_pools: HashMap<String, String>,
    deficit_weight: Float,
) -> SharedFixture {
    let mut fleet_builder = FleetBuilder::default();
    fleet_builder.add_driver(test_driver());
    for (key, _, end) in drivers {
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id(format!("v_{key}").as_str()).details(vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(*end) } }),
        }]);
        vehicle_builder.dimens_mut().set_driver_id((*key).to_string());
        fleet_builder.add_vehicle(vehicle_builder.build());
    }
    let fleet = fleet_builder.build();

    let actors: Vec<Arc<Actor>> =
        drivers.iter().map(|(key, ..)| get_test_actor_from_fleet(&fleet, format!("v_{key}").as_str())).collect();

    let transport = TestTransportCost::new_shared();
    let singles: Vec<Arc<Single>> = job_locations
        .iter()
        .enumerate()
        .map(|(idx, location)| {
            TestSingleBuilder::default().id(format!("job_{idx}").as_str()).location(Some(*location)).build_shared()
        })
        .collect();
    let job_list: Vec<Job> = singles.iter().cloned().map(Job::Single).collect();
    let jobs = Arc::new(Jobs::new(&fleet, job_list, transport.as_ref(), &test_logger()).unwrap());

    let anchors: HashMap<String, Vec<usize>> =
        drivers.iter().map(|(key, anchor, _)| ((*key).to_string(), vec![*anchor])).collect();

    let shared = TerritoryShared::new(
        transport,
        actors.clone(),
        jobs,
        Arc::new(|_: &Job, _: &Actor| true),
        TerritoryProximity::Distance,
        balance,
        0.0,
        deficit_weight,
        anchors,
        HashMap::new(),
        supplied_quotas,
        supplied_shares,
        quota_pools,
        Arc::new(|_: &Job| 1.0),
        false,
    );

    SharedFixture { shared, singles, actors }
}

/// Three drivers anchored on the same location are exactly equidistant from a job — a real tie in
/// the anchor ranking (mirrored anchors, a shared depot and a coarse matrix all produce one).
/// `scan_sorted_anchors` collects into a `HashMap` and sorts stably, so before the tie-break the
/// tied entries kept that map's hash order; the standard library gives the *n*-th map a process
/// builds its own seed, so the order moved with how many maps had been built earlier — which any
/// change to the work done before the solve moves in turn.
///
/// Every iteration below builds a fresh `seen` map, hence a fresh seed. The fleet is ordered d2,
/// d0, d1 so that neither hash order nor fleet order can pass by accident.
#[test]
fn tied_anchors_rank_by_driver_key_whatever_the_hash_seed() {
    let drivers = [("d2", 10, 1000.0), ("d0", 10, 1000.0), ("d1", 10, 1000.0)];
    let fixture = shared_over(&drivers, &[0], None, HashMap::new());
    let job = Job::Single(fixture.singles[0].clone());

    for _ in 0..64 {
        let ranking = fixture.shared.scan_sorted_anchors(0, &job);

        assert!(ranking.iter().all(|(_, prox)| *prox == 10.0), "the fixture must be an exact tie");
        assert_eq!(
            ranking.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>(),
            ["d0", "d1", "d2"],
            "tied drivers must rank by driver key"
        );
    }
}

/// The derived quota is `total_metric * cap / Σcaps`, and a float sum rounds by the order it is
/// folded in. `caps` is a `HashMap`, so summing it as it iterates made every quota depend on that
/// map's hash seed. The capacities disagree under reordering on purpose — `1e16 + 1` is still
/// `1e16`, while `1 + 1 + 1e16` gives `1e16 + 2`: real capacities rarely differ by sixteen orders
/// of magnitude, but the last-bit difference they do produce is the same difference, and only an
/// amplified one is assertable.
///
/// Each iteration builds a whole new `TerritoryShared`, so `caps` is a new map with a new seed.
#[test]
fn derived_quotas_do_not_depend_on_the_hash_seed() {
    let drivers = [("d0", 0usize, 1e16), ("d1", 10usize, 1.0), ("d2", 20usize, 1.0)];
    let quotas_once =
        || shared_over(&drivers, &[0, 10, 20], Some(TerritoryBalance::Activities), HashMap::new()).shared.quotas;

    let expected = quotas_once();
    assert_eq!(expected.len(), 3, "every driver must carry a derived quota");

    for _ in 0..64 {
        assert_eq!(quotas_once(), expected, "the derived quota must not move with the hash seed");
    }
}

/// PUSH sums `surplus × convexity × (distance to the nearest deficit anchor)` over the drivers, and
/// that sum went round the quota `HashMap` — so the solution's fitness, not just an internal, moved
/// with the map's hash seed. Three drivers are over quota (quota `0.0`, one job each) and only "d3"
/// is under (quota `10.0`, no jobs), so every source ships to "d3"'s anchor at 1. A quota of exactly
/// zero keeps the term linear (`convexity == 1`), so the numbers are the linear ones:
///
/// - "d0" is anchored 1e16 away, so its term is 1e16;
/// - "d1" (anchor 0) and "d2" (anchor 2) are one unit away, so their terms are 1 each.
///
/// Folded low-to-high that is `1e16 + 2`; folded high-first — which is what the pinned ascending
/// driver order gives — the two ones vanish under the leading term and the total is exactly `1e16`.
/// Same amplifier, and the same caveat, as the quota test above.
#[test]
fn push_total_does_not_depend_on_the_hash_seed() {
    const FAR: usize = 10_000_000_000_000_000;
    let drivers = [("d0", FAR, 1000.0), ("d1", 0usize, 1000.0), ("d2", 2usize, 1000.0), ("d3", 1usize, 1000.0)];
    let quotas = HashMap::from([
        ("d0".to_string(), 0.0),
        ("d1".to_string(), 0.0),
        ("d2".to_string(), 0.0),
        ("d3".to_string(), 10.0),
    ]);

    for _ in 0..64 {
        let fixture = shared_over(&drivers, &[FAR, 0, 2], Some(TerritoryBalance::Activities), quotas.clone());
        let solution = TestInsertionContextBuilder::default()
            .with_routes(
                fixture
                    .actors
                    .iter()
                    .take(3)
                    .zip(fixture.singles.iter())
                    .map(|(actor, single)| route_with(actor.clone(), single.clone(), 0))
                    .collect(),
            )
            .build();

        assert_eq!(fixture.shared.push(&solution.solution), 1e16, "the PUSH total must not move with the hash seed");
    }
}

/// A deficit costs nothing by default: a driver below its band is only a destination for somebody
/// else's surplus, never billed itself. That is what lets a shortfall concentrated on one technician
/// hide — work is conserved, so it reappears as a small surplus spread over the others, and every
/// one of those can sit inside its own band. Measured on a six-technician tenant, run 40: one technician 17% short
/// with nobody more than 10.6% over, and narrowing the band to 1% did not move it.
///
/// `deficit_weight` prices the shortfall itself. Here d0 carries both jobs against a quota of one
/// and d1 carries none, so the same one-job gap is a surplus on d0 and a deficit on d1: at weight
/// zero only the first is billed, and at weight one both are.
#[test]
fn a_deficit_costs_nothing_until_it_is_weighed() {
    let drivers = [("d0", 0usize, 1000.0), ("d1", 100usize, 1000.0)];
    let quotas = HashMap::from([("d0".to_string(), 1.0), ("d1".to_string(), 1.0)]);

    let push_at = |deficit_weight: Float| {
        let fixture = shared_over_with_shares(
            &drivers,
            &[0, 1],
            Some(TerritoryBalance::Activities),
            quotas.clone(),
            HashMap::new(),
            HashMap::new(),
            deficit_weight,
        );
        let solution = TestInsertionContextBuilder::default()
            .with_routes(vec![route_with_jobs(
                fixture.actors[0].clone(),
                vec![(fixture.singles[0].clone(), 0), (fixture.singles[1].clone(), 1)],
            )])
            .build();

        fixture.shared.push(&solution.solution)
    };

    let one_sided = push_at(0.0);
    let symmetric = push_at(1.0);

    assert!(one_sided > 0.0, "d0 is over its quota, so the surplus side must bill something");
    assert!(
        symmetric > one_sided,
        "weighing the deficit must add d1's shortfall to the bill: {symmetric} was not above {one_sided}"
    );
    assert_eq!(
        push_at(0.5),
        one_sided + (symmetric - one_sided) / 2.0,
        "the weight must scale the deficit side linearly"
    );
}

// endregion

/// `Distance` and `Duration` are properties of a ROUTE: travel depends on the order the stops are
/// visited, so no per-job term can express it. Both read a total the transport feature already
/// maintains — `get_total_distance()` is the whole route, and `get_paid_working_duration()` is the
/// paid span with the idle taken out, which is the part of it an assignment actually decides.
#[test]
fn route_load_measures_the_route_for_travel_targets() {
    for (balance, worked, distance, expected) in
        [(TerritoryBalance::Duration, 777.0, 999.0, 777.0), (TerritoryBalance::Distance, 777.0, 999.0, 999.0)]
    {
        let fixture = shared_over(&[("d0", 0, 1000.0)], &[5, 95], Some(balance), HashMap::new());
        let mut route_ctx = route_with_jobs(
            fixture.actors[0].clone(),
            vec![(fixture.singles[0].clone(), 5), (fixture.singles[1].clone(), 95)],
        );
        route_ctx.state_mut().set_paid_working_duration(worked);
        // Deliberately different, so a load reading the raw span instead of the worked part fails
        // here rather than in a campaign.
        route_ctx.state_mut().set_total_duration(worked * 3.0);
        route_ctx.state_mut().set_total_distance(distance);

        assert_eq!(fixture.shared.route_load(&route_ctx), expected, "{balance:?} must read the route's own total");
    }
}

/// The two counting targets stay per-job sums — a stop is a stop wherever it sits in the tour.
#[test]
fn route_load_still_sums_jobs_for_counting_targets() {
    let fixture = shared_over(&[("d0", 0, 1000.0)], &[5, 95], Some(TerritoryBalance::Activities), HashMap::new());
    let mut route_ctx = route_with_jobs(
        fixture.actors[0].clone(),
        vec![(fixture.singles[0].clone(), 5), (fixture.singles[1].clone(), 95)],
    );
    route_ctx.state_mut().set_total_duration(777.0);
    route_ctx.state_mut().set_total_distance(999.0);

    assert_eq!(fixture.shared.route_load(&route_ctx), 2.0, "two jobs are two activities");
}

/// `route_load` is what the balance MEASURES, but two places still need a per-job quantity in the
/// same unit: `compute_avg_metric`, which converts PUSH's surplus into "jobs' worth" so
/// `PUSH_CONVEXITY_GAIN` keeps its meaning, and `push_marginal`'s value factor. That quantity is
/// the job's ideal round trip from its nearest compatible vehicle start — never its distance to an
/// anchor, which is how far the old proxy was from real travel.
#[test]
fn job_metric_for_travel_targets_is_the_ideal_round_trip() {
    // Vehicle starts at 0, anchor sits ON the job at 5, so the anchor distance is 5 while the
    // round trip is 10.
    let fixture = shared_over(&[("d0", 5, 1000.0)], &[5], Some(TerritoryBalance::Distance), HashMap::new());
    let job = Job::Single(fixture.singles[0].clone());

    assert_eq!(fixture.shared.job_metric(&job), 10.0, "the estimate must be the round trip, not the anchor hop");
}

/// Shares carry the ratio, the solution carries the level, and a pool is what keeps one driver's
/// quota from being inflated by ground it may never reach.
#[test]
fn supplied_shares_take_their_level_from_their_own_pool() {
    let drivers = [("d1", 0, 1000.0), ("d2", 0, 1000.0), ("d3", 50, 1000.0)];
    let shares = HashMap::from([("d1".to_string(), 0.5), ("d2".to_string(), 0.5), ("d3".to_string(), 1.0)]);
    let pools = HashMap::from([
        ("d1".to_string(), "a".to_string()),
        ("d2".to_string(), "a".to_string()),
        ("d3".to_string(), "b".to_string()),
    ]);
    let fixture = shared_over_with_shares(
        &drivers,
        &[5, 95],
        Some(TerritoryBalance::Duration),
        HashMap::new(),
        shares,
        pools,
        0.0,
    );

    let loads = HashMap::from([("d1".to_string(), 80.0), ("d2".to_string(), 20.0), ("d3".to_string(), 40.0)]);
    let quotas = fixture.shared.effective_quotas(&loads);

    // Pool "a" holds 100 between two equal shares: 50 each, so d1 is over and d2 is under.
    assert_eq!(quotas.get("d1").copied(), Some(50.0));
    assert_eq!(quotas.get("d2").copied(), Some(50.0));
    // Pool "b" holds 40 and d3 owes all of it — untouched by pool "a"'s 100.
    assert_eq!(quotas.get("d3").copied(), Some(40.0));
    // The quotas of a pool sum to that pool's own total, which is what leaves a deficit facing
    // every surplus.
    assert_eq!(quotas["d1"] + quotas["d2"], loads["d1"] + loads["d2"]);
}

/// A driver the caller left out of the pool map still balances — against everyone else who was
/// left out. One default pool is the right answer when no hard gate splits the fleet.
#[test]
fn drivers_without_a_pool_share_one_default_pool() {
    let drivers = [("d1", 0, 1000.0), ("d2", 0, 1000.0)];
    let shares = HashMap::from([("d1".to_string(), 0.25), ("d2".to_string(), 0.75)]);
    let fixture = shared_over_with_shares(
        &drivers,
        &[5, 95],
        Some(TerritoryBalance::Distance),
        HashMap::new(),
        shares,
        HashMap::new(),
        0.0,
    );

    let loads = HashMap::from([("d1".to_string(), 60.0), ("d2".to_string(), 40.0)]);
    let quotas = fixture.shared.effective_quotas(&loads);

    assert_eq!(quotas.get("d1").copied(), Some(25.0));
    assert_eq!(quotas.get("d2").copied(), Some(75.0));
}

/// Shares outrank a supplied amount: a caller that sends both means the ratio it knows and a level
/// it could not.
#[test]
fn supplied_shares_replace_a_supplied_quota() {
    let drivers = [("d1", 0, 1000.0), ("d2", 0, 1000.0)];
    let fixture = shared_over_with_shares(
        &drivers,
        &[5, 95],
        Some(TerritoryBalance::Duration),
        HashMap::from([("d1".to_string(), 999.0), ("d2".to_string(), 999.0)]),
        HashMap::from([("d1".to_string(), 0.5), ("d2".to_string(), 0.5)]),
        HashMap::new(),
        0.0,
    );

    let loads = HashMap::from([("d1".to_string(), 30.0), ("d2".to_string(), 10.0)]);
    let quotas = fixture.shared.effective_quotas(&loads);

    assert_eq!(quotas.get("d1").copied(), Some(20.0), "the share's level wins over the supplied amount");
    assert_eq!(quotas.get("d2").copied(), Some(20.0));
}

/// `push_marginal` runs during insertion with only a route in hand, so the route's slice of its
/// driver's quota is cached by the solution-state pass. Under shares that cache is the ONLY place
/// the quota exists at all — its level is a property of a solution.
#[test]
fn the_solution_state_caches_each_route_s_slice_of_the_quota() {
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

    let feature = TerritoryFeatureBuilder::new("territory")
        .set_transport(transport)
        .set_actors(vec![actor_d0.clone(), actor_d1.clone()])
        .set_jobs(jobs)
        .set_compatibility_fn(|_, _| true)
        .set_proximity(TerritoryProximity::Distance)
        .set_balance(Some(TerritoryBalance::Duration))
        .set_anchors(HashMap::from([("d0".to_string(), vec![0usize]), ("d1".to_string(), vec![100usize])]))
        .set_quota_shares(HashMap::from([("d0".to_string(), 0.5), ("d1".to_string(), 0.5)]))
        .build()
        .unwrap();

    // d0 carries both jobs, d1 none: the pool's whole load sits on one driver.
    let mut ctx = TestInsertionContextBuilder::default()
        .with_routes(vec![
            route_with_jobs(actor_d0, vec![(job_near, 5), (job_far, 95)]),
            route_with_jobs(actor_d1, vec![]),
        ])
        .build();

    // Nothing is cached before the pass, and with shares there is no static quota to fall back on.
    assert!(ctx.solution.routes[0].state().get_territory_route_quota().is_none());

    feature.state.as_ref().unwrap().accept_solution_state(&mut ctx.solution);

    // Closed tour 0 -> 5 -> 95 -> 0 is 5 + 90 + 95 = 190, split by two equal shares.
    let loaded = ctx.solution.routes[0].state().get_territory_route_quota().copied().unwrap();
    let idle = ctx.solution.routes[1].state().get_territory_route_quota().copied().unwrap();
    assert_eq!(loaded, 95.0);
    assert_eq!(idle, 95.0, "the idle driver owes its share too, which is what makes it a deficit");
}

/// The `Duration` estimate has to carry the job's SERVICE time as well, because the route load it
/// estimates is the paid span — service plus idle plus the drive between jobs — and on a
/// field-service day the service dominates. Travel alone would price a two-hour visit next door
/// below a ten-minute visit across town.
#[test]
fn the_duration_estimate_carries_service_time_as_well_as_travel() {
    let mut builder = TestSingleBuilder::default();
    builder.id("job_near").location(Some(5)).duration(600.0);
    let job = Job::Single(builder.build_shared());

    let fleet = {
        let mut fleet_builder = FleetBuilder::default();
        fleet_builder.add_driver(test_driver());
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id("v_d0").details(vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
        }]);
        vehicle_builder.dimens_mut().set_driver_id("d0".to_string());
        fleet_builder.add_vehicle(vehicle_builder.build());
        fleet_builder.build()
    };
    let actor = get_test_actor_from_fleet(&fleet, "v_d0");
    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(Jobs::new(&fleet, vec![job.clone()], transport.as_ref(), &test_logger()).unwrap());

    let shared = TerritoryShared::new(
        transport,
        vec![actor],
        jobs,
        Arc::new(|_: &Job, _: &Actor| true),
        TerritoryProximity::Distance,
        Some(TerritoryBalance::Duration),
        0.0,
        // deficit_weight: zero keeps PUSH one-sided, which is what these assertions were measured against.
        0.0,
        HashMap::from([("d0".to_string(), vec![0usize])]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Arc::new(|_: &Job| 1.0),
        false,
    );

    // Round trip 0 -> 5 -> 0 is 10; the visit itself is 600.
    assert_eq!(shared.job_metric(&job), 610.0);
}

/// `Service` levels time spent at customers and nothing else — no travel in the load, none in the
/// estimate. It is the metric with no feedback: moving a job cannot change how long it takes.
#[test]
fn service_balances_time_at_customers_without_travel() {
    let mut builder = TestSingleBuilder::default();
    builder.id("job_0").location(Some(5)).duration(600.0);
    let job = Job::Single(builder.build_shared());

    let fleet = {
        let mut fleet_builder = FleetBuilder::default();
        fleet_builder.add_driver(test_driver());
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id("v_d0").details(vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
        }]);
        vehicle_builder.dimens_mut().set_driver_id("d0".to_string());
        fleet_builder.add_vehicle(vehicle_builder.build());
        fleet_builder.build()
    };
    let actor = get_test_actor_from_fleet(&fleet, "v_d0");
    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(Jobs::new(&fleet, vec![job.clone()], transport.as_ref(), &test_logger()).unwrap());

    let shared = TerritoryShared::new(
        transport,
        vec![actor.clone()],
        jobs,
        Arc::new(|_: &Job, _: &Actor| true),
        TerritoryProximity::Distance,
        Some(TerritoryBalance::Service),
        0.0,
        // deficit_weight: zero keeps PUSH one-sided, which is what these assertions were measured against.
        0.0,
        HashMap::from([("d0".to_string(), vec![0usize])]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Arc::new(|_: &Job| 1.0),
        false,
    );

    // The visit, and not the ten seconds of driving the round trip would add.
    assert_eq!(shared.job_metric(&job), 600.0);

    let route_ctx = route_with_jobs(actor, vec![(shared_single(&shared), 5)]);
    assert_eq!(shared.route_load(&route_ctx), 600.0, "the load is the sum over the tour's jobs");
}

/// The fixture's single job, for a test that needs it back out of the shared state.
fn shared_single(_shared: &TerritoryShared) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();
    builder.id("job_0").location(Some(5)).duration(600.0);
    builder.build_shared()
}

/// The two per-job questions are not the same question, and `Duration` answers them differently.
///
/// What a job is worth to the problem's TOTAL — which sizes a derived quota — is the travel to
/// reach it plus its service. What INSERTING it into a route costs is its service plus a detour,
/// and the detour is not the round trip this feature can see. Estimating the insertion at the round
/// trip made every job look alike and mis-scaled the marginal against PULL.
#[test]
fn a_duration_job_is_worth_its_round_trip_to_the_total_and_its_service_to_a_route() {
    let mut builder = TestSingleBuilder::default();
    builder.id("job_0").location(Some(5)).duration(600.0);
    let job = Job::Single(builder.build_shared());

    let fleet = {
        let mut fleet_builder = FleetBuilder::default();
        fleet_builder.add_driver(test_driver());
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id("v_d0").details(vec![VehicleDetail {
            start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
        }]);
        vehicle_builder.dimens_mut().set_driver_id("d0".to_string());
        fleet_builder.add_vehicle(vehicle_builder.build());
        fleet_builder.build()
    };
    let actor = get_test_actor_from_fleet(&fleet, "v_d0");
    let transport = TestTransportCost::new_shared();
    let jobs = Arc::new(Jobs::new(&fleet, vec![job.clone()], transport.as_ref(), &test_logger()).unwrap());

    let shared = TerritoryShared::new(
        transport,
        vec![actor],
        jobs,
        Arc::new(|_: &Job, _: &Actor| true),
        TerritoryProximity::Distance,
        Some(TerritoryBalance::Duration),
        0.0,
        // deficit_weight: zero keeps PUSH one-sided, which is what these assertions were measured against.
        0.0,
        HashMap::from([("d0".to_string(), vec![0usize])]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        Arc::new(|_: &Job| 1.0),
        false,
    );

    // Round trip 0 -> 5 -> 0 is 10, the visit is 600.
    assert_eq!(shared.job_metric(&job), 610.0, "the total keeps the travel");
    assert_eq!(shared.insertion_metric(&job), 600.0, "the marginal does not");
}
