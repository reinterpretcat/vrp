use super::*;
use crate::construction::features::create_minimize_tours_feature;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::{
    create_default_refinement_ctx, generate_matrix_routes_with_defaults, promote_to_locked, rearrange_jobs_in_routes,
};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use rosomaxa::prelude::HeuristicObjective;
use std::sync::Arc;

fn create_insertion_ctx(job_order: &[Vec<&str>]) -> InsertionContext {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 64])));
    let (problem, solution) = generate_matrix_routes_with_defaults(3, 2, true);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order);

    insertion_ctx
}

#[test]
fn can_improve_solution_with_one_way_relocation() {
    let insertion_ctx = create_insertion_ctx(&[vec!["c0", "c1", "c2", "c3"], vec!["c4", "c5"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = RelocateInterRoute::default().explore(&refinement_ctx, &insertion_ctx).expect("no improvement");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    assert_eq!(
        get_customer_ids_from_routes(&result),
        vec![
            vec!["c0".to_string(), "c1".to_string(), "c2".to_string()],
            vec!["c3".to_string(), "c4".to_string(), "c5".to_string()]
        ]
    );
}

#[test]
fn can_remove_empty_source_route() {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 64])));
    let (mut problem, solution) = generate_matrix_routes_with_defaults(3, 2, true);
    problem.goal = Arc::new(
        TestGoalContextBuilder::empty().add_feature(create_minimize_tours_feature("minimize_tours").unwrap()).build(),
    );
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, &[vec!["c0"], vec!["c1", "c2", "c3", "c4", "c5"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = RelocateInterRoute::default().explore(&refinement_ctx, &insertion_ctx).expect("no improvement");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    assert_eq!(result.solution.routes.len(), 1);
}

#[test]
fn does_not_return_non_improving_relocation() {
    let insertion_ctx = promote_to_locked(
        create_insertion_ctx(&[vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"]]),
        &["c1", "c2", "c3", "c4", "c5"],
    );
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = RelocateInterRoute::default().explore(&refinement_ctx, &insertion_ctx);

    assert!(result.is_none());
}
