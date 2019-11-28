use std::sync::Arc;

use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::get_sorted_customer_ids_from_jobs;
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::utils::random::FakeRandom;
use crate::refinement::ruin::{Ruin, WorstJobRemoval};
use crate::refinement::RefinementContext;

#[test]
fn can_ruin_worst_jobs() {
    // TODO
    let matrix = (8, 3);
    let ints = vec![0, 3, 1, 2];
    let reals = vec![1., 5.];
    let expected_ids = vec!["c1", "c2", "c3", "c4", "c5"];

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let insertion_ctx: InsertionContext = InsertionContext::new_from_solution(
        Arc::new(problem),
        (Arc::new(solution), None),
        Arc::new(FakeRandom::new(ints, reals)),
    );

    let insertion_ctx = WorstJobRemoval::default().run(
        &RefinementContext { problem: insertion_ctx.problem.clone(), population: vec![], generation: 0 },
        insertion_ctx,
    );

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), expected_ids);
}
