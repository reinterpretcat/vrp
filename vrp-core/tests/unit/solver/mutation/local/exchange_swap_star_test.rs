use super::*;
use crate::helpers::solver::*;
use crate::utils::Environment;

#[test]
fn can_use_exchange_swap_star() {
    let locked_ids = &[];

    let matrix = (3, 3);
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone()),
        locked_ids,
    );

    let _ = ExchangeSwapStar::new(environment.random.clone())
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");
}
