use super::*;
use crate::construction::features::{TransportFeatureBuilder, create_minimize_tours_feature};
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::{
    create_default_refinement_ctx, generate_matrix_routes_with_defaults, promote_to_locked, rearrange_jobs_in_routes,
};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::ViolationCode;
use rosomaxa::prelude::Quota;
use std::sync::Arc;

struct ReachedQuota;

impl Quota for ReachedQuota {
    fn is_reached(&self) -> bool {
        true
    }
}

fn create_insertion_ctx_with_size(rows: usize, cols: usize, job_order: &[Vec<&str>]) -> InsertionContext {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 128])));
    let (mut problem, solution) = generate_matrix_routes_with_defaults(rows, cols, true);
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
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order);

    insertion_ctx
}

fn create_insertion_ctx(job_order: &[Vec<&str>]) -> InsertionContext {
    create_insertion_ctx_with_size(4, 2, job_order)
}

fn create_search(move_types: MoveTypes) -> ExchangeSequenceBest {
    ExchangeSequenceBest::with_move_types(move_types)
}

#[test]
fn can_relocate_two_jobs() {
    let insertion_ctx =
        create_insertion_ctx_with_size(3, 3, &[vec!["c0", "c1", "c2", "c6", "c7"], vec!["c3", "c4", "c5"], vec!["c8"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search =
        create_search(MoveTypes { relocate: true, exchange_two_with_one: false, exchange_two_with_two: false });

    let routes =
        insertion_ctx.solution.routes.iter().map(|route_ctx| get_ordered_jobs(route_ctx.route())).collect::<Vec<_>>();
    let desired = SequenceMove::Relocate {
        source_route_idx: 0,
        target_route_idx: 2,
        jobs: [routes[0][3].clone(), routes[0][4].clone()],
        anchor: routes[2][0].clone(),
        position: RelativePosition::Before,
    };
    let direct = apply_sequence_move(&insertion_ctx, desired).expect("cannot apply desired relocation");
    assert_eq!(insertion_ctx.problem.goal.total_order(&direct, &insertion_ctx), Ordering::Less);

    let result = search.explore(&refinement_ctx, &insertion_ctx).expect("no sequence relocation");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
}

#[test]
fn can_exchange_two_jobs_with_one() {
    let insertion_ctx = create_insertion_ctx(&[vec!["c1", "c2", "c3", "c4", "c5"], vec!["c0", "c6", "c7"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search =
        create_search(MoveTypes { relocate: false, exchange_two_with_one: true, exchange_two_with_two: false });

    let routes =
        insertion_ctx.solution.routes.iter().map(|route_ctx| get_ordered_jobs(route_ctx.route())).collect::<Vec<_>>();
    let desired = SequenceMove::Exchange {
        first_route_idx: 0,
        second_route_idx: 1,
        first_jobs: [routes[0][3].clone(), routes[0][4].clone()],
        second_jobs: JobSequence::One(routes[1][0].clone()),
    };
    let direct = apply_sequence_move(&insertion_ctx, desired).expect("cannot apply desired exchange");
    assert_eq!(insertion_ctx.problem.goal.total_order(&direct, &insertion_ctx), Ordering::Less);

    let result = search.explore(&refinement_ctx, &insertion_ctx).expect("no 2-for-1 exchange");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
}

#[test]
fn can_exchange_two_jobs_with_two() {
    let insertion_ctx = create_insertion_ctx(&[vec!["c0", "c1", "c6", "c7"], vec!["c4", "c5", "c2", "c3"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search =
        create_search(MoveTypes { relocate: false, exchange_two_with_one: false, exchange_two_with_two: true });

    let result = search.explore(&refinement_ctx, &insertion_ctx).expect("no 2-for-2 exchange");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    let mut actual = get_customer_ids_from_routes(&result);
    actual.sort();
    assert_eq!(
        actual,
        vec![
            vec!["c0".to_string(), "c1".to_string(), "c2".to_string(), "c3".to_string()],
            vec!["c4".to_string(), "c5".to_string(), "c6".to_string(), "c7".to_string()]
        ]
    );
}

#[test]
fn does_not_move_locked_jobs() {
    let insertion_ctx = promote_to_locked(
        create_insertion_ctx(&[vec!["c0", "c1", "c6", "c7"], vec!["c4", "c5", "c2", "c3"]]),
        &["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7"],
    );
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(ExchangeSequenceBest::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn uses_configured_objective_for_acceptance() {
    let mut insertion_ctx = create_insertion_ctx_with_size(2, 2, &[vec!["c0", "c1"], vec!["c2", "c3"]]);
    let problem = &insertion_ctx.problem;
    insertion_ctx.problem = Arc::new(crate::models::Problem {
        fleet: problem.fleet.clone(),
        jobs: problem.jobs.clone(),
        locks: problem.locks.clone(),
        goal: Arc::new(
            TestGoalContextBuilder::empty().add_feature(create_minimize_tours_feature("tours").unwrap()).build(),
        ),
        activity: problem.activity.clone(),
        transport: problem.transport.clone(),
        extras: problem.extras.clone(),
    });
    insertion_ctx.restore();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search =
        create_search(MoveTypes { relocate: true, exchange_two_with_one: false, exchange_two_with_two: false });

    let result = search.explore(&refinement_ctx, &insertion_ctx).expect("no route-eliminating relocation");

    assert_eq!(result.solution.routes.len(), 1);
    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
}

#[test]
fn can_stop_on_reached_quota() {
    let mut insertion_ctx = create_insertion_ctx(&[vec!["c0", "c1", "c6", "c7"], vec!["c4", "c5", "c2", "c3"]]);
    Arc::make_mut(&mut insertion_ctx.environment).quota = Some(Arc::new(ReachedQuota));
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(ExchangeSequenceBest::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}
