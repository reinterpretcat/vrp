use super::*;
use crate::algorithms::geometry::Point;
use crate::construction::features::TransportFeatureBuilder;
use crate::helpers::models::domain::{TestGoalContextBuilder, get_customer_ids_from_routes};
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::{
    create_default_refinement_ctx, generate_matrix_distances_from_points, generate_matrix_routes, promote_to_locked,
    rearrange_jobs_in_routes,
};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::{FeatureBuilder, FeatureObjective, ViolationCode};
use rosomaxa::prelude::Quota;
use std::sync::Arc;

struct PreferRouteOrder(Vec<String>);

impl FeatureObjective for PreferRouteOrder {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        if get_customer_ids_from_routes(solution).first() == Some(&self.0) { 0. } else { 1. }
    }

    fn estimate(&self, _: &MoveContext<'_>) -> Cost {
        Cost::default()
    }
}

struct ReachedQuota;

impl Quota for ReachedQuota {
    fn is_reached(&self) -> bool {
        true
    }
}

fn create_insertion_ctx() -> InsertionContext {
    let points = [(0., 0.), (1., 0.), (2., 0.), (3., 0.), (4., 0.)];
    let distances =
        generate_matrix_distances_from_points(&points.iter().map(|&(x, y)| Point::new(x, y)).collect::<Vec<_>>());
    let (mut problem, solution) = generate_matrix_routes(
        points.len() - 1,
        1,
        false,
        |_, _, _| TestGoalContextBuilder::default().build(),
        |id, location| {
            TestSingleBuilder::default().id(id).location(location.map(|location| location + 1)).build_shared()
        },
        |vehicle| vehicle,
        |_| (distances.clone(), distances.clone()),
    );
    problem.goal = Arc::new(
        TestGoalContextBuilder::empty()
            .add_feature(
                TransportFeatureBuilder::new("transport")
                    .set_violation_code(ViolationCode(1))
                    .set_transport_cost(problem.transport.clone())
                    .set_activity_cost(problem.activity.clone())
                    .build_minimize_cost()
                    .unwrap(),
            )
            .build(),
    );
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![])));
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, &[vec!["c0", "c2", "c1", "c3"]]);

    insertion_ctx
}

#[test]
fn can_relocate_single_job_within_route() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result =
        RelocateIntraRoute::default().explore(&refinement_ctx, &insertion_ctx).expect("no single-job relocation");

    assert_ne!(get_customer_ids_from_routes(&result), get_customer_ids_from_routes(&insertion_ctx));
    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    assert!(result.solution.required.is_empty());
    assert!(result.solution.unassigned.is_empty());
    assert_eq!(
        result.solution.registry.resources().available().count(),
        insertion_ctx.solution.registry.resources().available().count()
    );
}

#[test]
fn uses_configured_objective_for_acceptance() {
    let mut insertion_ctx = create_insertion_ctx();
    let expected = get_customer_ids_from_routes(&insertion_ctx).into_iter().next().unwrap();
    let problem = &insertion_ctx.problem;
    insertion_ctx.problem = Arc::new(crate::models::Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        goal: Arc::new(
            TestGoalContextBuilder::empty()
                .add_feature(
                    FeatureBuilder::default()
                        .with_name("route_order")
                        .with_objective(PreferRouteOrder(expected))
                        .build()
                        .unwrap(),
                )
                .add_feature(
                    TransportFeatureBuilder::new("transport")
                        .set_violation_code(ViolationCode(1))
                        .set_transport_cost(problem.transport.clone())
                        .set_activity_cost(problem.activity.clone())
                        .build_minimize_cost()
                        .unwrap(),
                )
                .build(),
        ),
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        extras: problem.extras.clone(),
    });
    insertion_ctx.restore();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(RelocateIntraRoute::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn does_not_move_locked_jobs() {
    let insertion_ctx = promote_to_locked(create_insertion_ctx(), &["c0", "c1", "c2", "c3"]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(RelocateIntraRoute::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn can_stop_on_reached_quota() {
    let mut insertion_ctx = create_insertion_ctx();
    Arc::make_mut(&mut insertion_ctx.environment).quota = Some(Arc::new(ReachedQuota));
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(RelocateIntraRoute::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}
