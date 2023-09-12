use super::*;
use crate::helpers::construction::heuristics::create_test_insertion_context;
use crate::helpers::models::solution::create_test_registry;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};

#[test]
fn can_add_extra_constraint() {
    let original_ctx = create_test_insertion_context(create_test_registry());
    let target_ctx = create_target_insertion_ctx(&original_ctx, 1..3, 4..8);

    assert_eq!(original_ctx.problem.goal.constraints().count() + 1, target_ctx.problem.goal.constraints().count());
}

#[test]
fn can_remove_jobs() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 6, false);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    let orig_total_jobs: usize =
        insertion_ctx.solution.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum();

    let jobs = remove_jobs(&mut insertion_ctx, 2..3, 4..8);

    assert!(!jobs.is_empty());
    let result_total_jobs: usize =
        insertion_ctx.solution.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum();
    assert!(result_total_jobs < orig_total_jobs);
    assert_eq!(insertion_ctx.solution.unassigned.len(), orig_total_jobs - result_total_jobs);
}

#[test]
fn can_restore_constraints_in_context() {
    let original_ctx = create_test_insertion_context(create_test_registry());
    let recreate = Arc::new(RecreateWithCheapest::new(original_ctx.environment.random.clone()));
    let heuristic = RedistributeSearch::new(recreate);

    let result_ctx = heuristic.search(&create_default_refinement_ctx(original_ctx.problem.clone()), &original_ctx);

    assert_eq!(original_ctx.problem.goal.constraints().count(), result_ctx.problem.goal.constraints().count());
}
