use crate::construction::features::create_period_balanced_feature;
use crate::construction::heuristics::{MoveContext, RouteContext, RouteState};
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{TestSingleBuilder, TestVehicleBuilder, test_driver};
use crate::helpers::models::solution::ActivityBuilder;
use crate::models::common::{TimeInterval, TimeWindow};
use crate::models::problem::{Actor, ActorDetail, Job, VehiclePlace};
use std::collections::HashMap;
use std::sync::Arc;

/// Groups are keyed by the actor's start location so tests can control group membership
/// without wiring vehicle-id dimensions.
fn group_key_by_start(actor: &Actor) -> Option<String> {
    actor.detail.start.as_ref().map(|start| format!("g{}", start.location))
}

fn metric_by_activity_count(route_ctx: &RouteContext) -> crate::models::common::Cost {
    route_ctx.route().tour.job_activity_count() as crate::models::common::Cost
}

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

/// Builds a closed route on `actor` carrying `job_count` delivery activities, so that
/// `job_activity_count()` (the metric used by these tests) equals `job_count`.
fn build_route_with_jobs(actor: Arc<Actor>, job_count: usize) -> RouteContext {
    let start_loc = actor.detail.start.as_ref().unwrap().location;
    let end_loc = actor.detail.end.as_ref().unwrap().location;
    let route = crate::models::solution::Route {
        actor,
        tour: {
            let mut tour = crate::models::solution::Tour::default();
            tour.set_start(ActivityBuilder::with_location(start_loc).job(None).build());
            tour.set_end(ActivityBuilder::with_location(end_loc).job(None).build());
            for i in 0..job_count {
                let job = TestSingleBuilder::default().location(Some(start_loc + i + 1)).build_shared();
                tour.insert_last(ActivityBuilder::with_location(start_loc + i + 1).job(Some(job)).build());
            }
            tour
        },
    };
    RouteContext::new_with_state(route, RouteState::default())
}

#[test]
fn fitness_scale_returns_the_reference() {
    // The scalarizing multi-objective divides fitness/estimate by fitness_scale(); the
    // period-balance objective must expose its per-problem reference here (not the raw 1.0)
    // so that its normalized contribution is comparable to compact/vehicle-distance.
    let feature = create_period_balanced_feature(
        "test_period_balance",
        HashMap::from([("g0".to_string(), 1usize)]),
        group_key_by_start,
        metric_by_activity_count,
        42.0,
    )
    .unwrap();
    let objective = feature.objective.unwrap();

    assert_eq!(objective.fitness_scale(), 42.0);
}

#[test]
fn estimate_is_route_metric_per_shift_capacity() {
    // Insertion guidance must be the route's per-shift load (metric / group capacity), in the
    // same unit as fitness, so that dividing by the reference yields a dimensionless term.
    // Route carries 4 job activities; its group has capacity 2 -> estimate 4 / 2 = 2.
    let feature = create_period_balanced_feature(
        "test_period_balance",
        HashMap::from([("g0".to_string(), 2usize)]),
        group_key_by_start,
        metric_by_activity_count,
        1.0,
    )
    .unwrap();
    let objective = feature.objective.unwrap();

    let route_ctx = build_route_with_jobs(create_actor_at(0), 4);
    let job = Job::Single(TestSingleBuilder::default().location(Some(50)).build_shared());
    let insertion_ctx = TestInsertionContextBuilder::default().build();

    let estimate = objective.estimate(&MoveContext::route(&insertion_ctx.solution, &route_ctx, &job));

    assert_eq!(estimate, 2.0);
}

#[test]
fn fitness_is_standard_deviation_of_per_shift_ratios() {
    // Two employees: group g0 has capacity 1 and 2 activities (ratio 2), group g100 has
    // capacity 2 and 2 activities (ratio 1). Population stddev of [2, 1] is 0.5.
    // A coefficient-of-variation implementation would instead report 0.5 / 1.5 = 0.333...,
    // so asserting 0.5 discriminates stddev from CV.
    let feature = create_period_balanced_feature(
        "test_period_balance",
        HashMap::from([("g0".to_string(), 1usize), ("g100".to_string(), 2usize)]),
        group_key_by_start,
        metric_by_activity_count,
        1.0,
    )
    .unwrap();
    let objective = feature.objective.unwrap();

    let route_g0 = build_route_with_jobs(create_actor_at(0), 2);
    let route_g100 = build_route_with_jobs(create_actor_at(100), 2);
    let insertion_ctx =
        TestInsertionContextBuilder::default().with_routes(vec![route_g0, route_g100]).build();

    let fitness = objective.fitness(&insertion_ctx);

    assert!((fitness - 0.5).abs() < 1e-9, "expected stddev 0.5, got {fitness}");
}
