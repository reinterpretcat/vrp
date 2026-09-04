use crate::construction::features::VehicleDistanceFeatureBuilder;
use crate::construction::heuristics::MoveContext;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{TestSingleBuilder, TestTransportCost, TestVehicleBuilder, test_driver};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder, RouteContextBuilder};
use crate::models::common::{TimeInterval, TimeWindow};
use crate::models::problem::{Actor, ActorDetail, Job, VehiclePlace};
use crate::models::{Feature, FeatureState};
use std::sync::Arc;

fn create_actor_at(location: usize) -> Arc<Actor> {
    let vehicle = TestVehicleBuilder::default()
        .id(&format!("v_{location}"))
        .details(vec![crate::models::problem::VehicleDetail {
            start: Some(VehiclePlace { location, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
        }])
        .build();

    Arc::new(Actor {
        vehicle: Arc::new(vehicle),
        driver: Arc::new(test_driver()),
        detail: ActorDetail {
            start: Some(VehiclePlace { location, time: TimeInterval { earliest: Some(0.0), latest: None } }),
            end: Some(VehiclePlace { location, time: TimeInterval { earliest: None, latest: Some(1000.0) } }),
            time: TimeWindow { start: 0.0, end: 1000.0 },
        },
    })
}

fn create_test_feature(actors: Vec<Arc<Actor>>) -> crate::models::Feature {
    VehicleDistanceFeatureBuilder::new("test_vehicle_distance")
        .set_transport(TestTransportCost::new_shared())
        .set_actors(actors)
        .set_compatibility_fn(|_, _| true)
        .build()
        .unwrap()
}

// ============================================================================
// Builder Tests
// ============================================================================

#[test]
fn can_create_feature_with_all_required_parameters() {
    let actors = vec![create_actor_at(0)];
    let result = VehicleDistanceFeatureBuilder::new("test")
        .set_transport(TestTransportCost::new_shared())
        .set_actors(actors)
        .set_compatibility_fn(|_, _| true)
        .build();

    assert!(result.is_ok());
    let feature = result.unwrap();
    assert!(feature.objective.is_some());
    assert!(feature.state.is_some());
}

#[test]
fn can_return_error_when_transport_not_set() {
    let actors = vec![create_actor_at(0)];
    let result =
        VehicleDistanceFeatureBuilder::new("test").set_actors(actors).set_compatibility_fn(|_, _| true).build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("transport"));
}

#[test]
fn can_return_error_when_actors_not_set() {
    let result = VehicleDistanceFeatureBuilder::new("test")
        .set_transport(TestTransportCost::new_shared())
        .set_compatibility_fn(|_, _| true)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("actors"));
}

#[test]
fn can_return_error_when_compatibility_fn_not_set() {
    let actors = vec![create_actor_at(0)];
    let result = VehicleDistanceFeatureBuilder::new("test")
        .set_transport(TestTransportCost::new_shared())
        .set_actors(actors)
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("compatibility_fn"));
}

// ============================================================================
// Fitness Tests - verify penalty calculations
// ============================================================================

#[test]
fn can_return_zero_fitness_when_job_on_nearest_vehicle() {
    // Vehicle at 0, job at 5. Only one vehicle, so assigned == nearest.
    let actor = create_actor_at(0);
    let actors = vec![actor.clone()];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();

    let job = TestSingleBuilder::default().location(Some(5)).build_shared();
    let route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_start(ActivityBuilder::with_location(0).job(None).build())
                .with_end(ActivityBuilder::with_location(0).job(None).build())
                .add_activity(ActivityBuilder::with_location(5).job(Some(job)).build())
                .build(),
        )
        .build();
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 0.0);
}

#[test]
fn can_return_penalty_when_job_on_farther_vehicle() {
    // Two vehicles: v0 at 0, v1 at 100. Job at 5, assigned to v1 (at 100).
    // Round-trip metric: 2 * |a - b| (test transport is symmetric).
    //   round_trip(v1, job) = 2 * 95 = 190
    //   round_trip(v0, job) = 2 * 5  =  10  (nearest)
    //   penalty = 190 - 10 = 180
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();

    let job = TestSingleBuilder::default().location(Some(5)).build_shared();
    // Build route using actor_100 (vehicle starting at 100)
    let route = crate::models::solution::Route {
        actor: actor_100,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(100).job(None).build());
            tour.set_end(ActivityBuilder::with_location(100).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(5).job(Some(job)).build());
            tour
        },
    };
    let route_ctx = crate::construction::heuristics::RouteContext::new_with_state(
        route,
        crate::construction::heuristics::RouteState::default(),
    );
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 180.0);
}

#[test]
fn can_return_zero_fitness_for_empty_route() {
    let actors = vec![create_actor_at(0)];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();
    let route_ctx = RouteContextBuilder::default().build();
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 0.0);
}

#[test]
fn can_sum_penalties_across_multiple_jobs() {
    // Two vehicles: at 0 and at 100. Two jobs at 5 and 10, both assigned to v100.
    // Round-trip metric (test transport is symmetric, so round_trip = 2 * one-way):
    //   job@5  → assigned 2*95=190, nearest 2*5=10,   penalty=180
    //   job@10 → assigned 2*90=180, nearest 2*10=20,  penalty=160
    //   total = 340
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();

    let job1 = TestSingleBuilder::default().location(Some(5)).build_shared();
    let job2 = TestSingleBuilder::default().location(Some(10)).build_shared();
    let route = crate::models::solution::Route {
        actor: actor_100,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(100).job(None).build());
            tour.set_end(ActivityBuilder::with_location(100).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(5).job(Some(job1)).build());
            tour.insert_last(ActivityBuilder::with_location(10).job(Some(job2)).build());
            tour
        },
    };
    let route_ctx = crate::construction::heuristics::RouteContext::new_with_state(
        route,
        crate::construction::heuristics::RouteState::default(),
    );
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 340.0);
}

// ============================================================================
// Estimate Tests - verify construction-time guidance
// ============================================================================

#[test]
fn can_estimate_zero_when_inserting_into_nearest_vehicle() {
    // Two vehicles: at 0 and at 100. Inserting job at 5 into v0's route.
    // dist(5, 0) = 5 (assigned), dist(5, 0) = 5 (nearest) -> penalty = 0
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0.clone(), actor_100];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();

    let job = Job::Single(TestSingleBuilder::default().location(Some(5)).build_shared());
    let route = crate::models::solution::Route {
        actor: actor_0,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(0).job(None).build());
            tour.set_end(ActivityBuilder::with_location(0).job(None).build());
            tour
        },
    };
    let route_ctx = crate::construction::heuristics::RouteContext::new_with_state(
        route,
        crate::construction::heuristics::RouteState::default(),
    );
    let insertion_ctx = TestInsertionContextBuilder::default().build();

    let estimate = objective.estimate(&MoveContext::route(&insertion_ctx.solution, &route_ctx, &job));
    assert_eq!(estimate, 0.0);
}

#[test]
fn can_estimate_penalty_when_inserting_into_farther_vehicle() {
    // Two vehicles: at 0 and at 100. Inserting job at 5 into v100's route.
    // Round-trip (symmetric test transport): 2*95=190 assigned vs 2*5=10 nearest → penalty 180
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let objective = feature.objective.unwrap();

    let job = Job::Single(TestSingleBuilder::default().location(Some(5)).build_shared());
    let route = crate::models::solution::Route {
        actor: actor_100,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(100).job(None).build());
            tour.set_end(ActivityBuilder::with_location(100).job(None).build());
            tour
        },
    };
    let route_ctx = crate::construction::heuristics::RouteContext::new_with_state(
        route,
        crate::construction::heuristics::RouteState::default(),
    );
    let insertion_ctx = TestInsertionContextBuilder::default().build();

    let estimate = objective.estimate(&MoveContext::route(&insertion_ctx.solution, &route_ctx, &job));
    assert_eq!(estimate, 180.0);
}

// ============================================================================
// Comparison Tests
// ============================================================================

#[test]
fn can_prefer_route_with_jobs_near_vehicle_start() {
    // Two vehicles at 0 and 100. Two routes:
    // Route A: v0 with job at 5 (near start) → round-trip 2*5 vs 2*5, penalty 0
    // Route B: v0 with job at 95 (far from v0, near v100) → 2*95 vs 2*5, penalty 180
    // Route A should have lower fitness.
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0.clone(), actor_100];
    let feature_a = create_test_feature(actors.clone());
    let feature_b = create_test_feature(actors);
    let obj_a = feature_a.objective.unwrap();
    let obj_b = feature_b.objective.unwrap();

    // Route A: job at 5, on v0
    let job_near = TestSingleBuilder::default().location(Some(5)).build_shared();
    let route_a = crate::models::solution::Route {
        actor: actor_0.clone(),
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(0).job(None).build());
            tour.set_end(ActivityBuilder::with_location(0).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(5).job(Some(job_near)).build());
            tour
        },
    };
    let route_ctx_a = crate::construction::heuristics::RouteContext::new_with_state(
        route_a,
        crate::construction::heuristics::RouteState::default(),
    );
    let ctx_a = TestInsertionContextBuilder::default().with_routes(vec![route_ctx_a]).build();
    let fitness_a = obj_a.fitness(&ctx_a);

    // Route B: job at 95, on v0 (far from v0's start, closer to v100)
    let job_far = TestSingleBuilder::default().location(Some(95)).build_shared();
    let route_b = crate::models::solution::Route {
        actor: actor_0,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(0).job(None).build());
            tour.set_end(ActivityBuilder::with_location(0).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(95).job(Some(job_far)).build());
            tour
        },
    };
    let route_ctx_b = crate::construction::heuristics::RouteContext::new_with_state(
        route_b,
        crate::construction::heuristics::RouteState::default(),
    );
    let ctx_b = TestInsertionContextBuilder::default().with_routes(vec![route_ctx_b]).build();
    let fitness_b = obj_b.fitness(&ctx_b);

    assert_eq!(fitness_a, 0.0);
    assert_eq!(fitness_b, 180.0);
    assert!(fitness_a < fitness_b);
}

// ============================================================================
// Pipeline Integration Tests
//
// These reproduce the production bug class where `fitness()` returned 0 because
// the per-route cache had been written once for empty routes and never refreshed
// after jobs were inserted. They exercise the FeatureState lifecycle directly,
// rather than only the `compute_route_penalty` helper.
// ============================================================================

fn build_route_with_job(actor: Arc<Actor>, job_location: usize) -> crate::construction::heuristics::RouteContext {
    let job = TestSingleBuilder::default().location(Some(job_location)).build_shared();
    let route = crate::models::solution::Route {
        actor: actor.clone(),
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            let start_loc = actor.detail.start.as_ref().unwrap().location;
            let end_loc = actor.detail.end.as_ref().unwrap().location;
            tour.set_start(ActivityBuilder::with_location(start_loc).job(None).build());
            tour.set_end(ActivityBuilder::with_location(end_loc).job(None).build());
            tour.insert_last(ActivityBuilder::with_location(job_location).job(Some(job)).build());
            tour
        },
    };
    crate::construction::heuristics::RouteContext::new_with_state(
        route,
        crate::construction::heuristics::RouteState::default(),
    )
}

fn extract_state_and_objective(feature: Feature) -> (Arc<dyn FeatureState>, Arc<dyn crate::models::FeatureObjective>) {
    (feature.state.unwrap(), feature.objective.unwrap())
}

#[test]
fn accept_solution_state_repopulates_after_tour_mutation() {
    // Regression test for the production bug:
    //   1. Initial empty routes have per-route cache populated to penalty=0.
    //   2. Jobs are inserted into routes (tour mutated), but the per-route cache
    //      is not refreshed.
    //   3. accept_solution_state must rebuild the per-route cache from the
    //      current tours — not skip routes based on a stale flag — so that
    //      fitness() returns the correct excess-distance total.
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let (state, objective) = extract_state_and_objective(feature);

    // Route on v100 with a job at 5 (closer to v0). Round-trip: 2*95 vs 2*5 → penalty 180.
    let route_ctx = build_route_with_job(actor_100, 5);
    let mut insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    // Pretend the pipeline never told us about the insertion: the route's
    // per-route cache is empty (None). accept_solution_state must still produce
    // the correct fitness.
    state.accept_solution_state(&mut insertion_ctx.solution);
    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 180.0);
}

#[test]
fn accept_insertion_eagerly_updates_per_route_cache() {
    // accept_insertion must refresh the cache immediately so callers that read
    // fitness between insertions (e.g. NSGA dominance comparisons) see the
    // up-to-date penalty without waiting for the next accept_solution_state.
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let (state, objective) = extract_state_and_objective(feature);

    let route_ctx = build_route_with_job(actor_100, 5);
    let job = Job::Single(route_ctx.route().tour.all_activities().nth(1).unwrap().job.as_ref().unwrap().clone());
    let mut insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    state.accept_insertion(&mut insertion_ctx.solution, 0, &job);
    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 180.0);
}

#[test]
fn fitness_recovers_when_per_route_cache_is_absent() {
    // Defense-in-depth: even if some future pipeline forgets to call our
    // accept_* methods, fitness() falls back to a live recompute per route
    // rather than reporting 0.
    let actor_0 = create_actor_at(0);
    let actor_100 = create_actor_at(100);
    let actors = vec![actor_0, actor_100.clone()];
    let feature = create_test_feature(actors);
    let (_, objective) = extract_state_and_objective(feature);

    let route_ctx = build_route_with_job(actor_100, 5);
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

    // Note: state.accept_* was never called. Per-route cache is empty.
    let fitness = objective.fitness(&insertion_ctx);
    assert_eq!(fitness, 180.0);
}
