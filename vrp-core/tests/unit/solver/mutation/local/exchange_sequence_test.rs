use super::*;
use crate::helpers::solver::{generate_matrix_routes_with_defaults, promote_to_locked};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use std::sync::Arc;

#[test]
fn can_extract_jobs() {
    let route_idx = 1;
    let sequence_size = 3;
    let ints = vec![2];
    let reals = vec![];

    let matrix = (5, 3);
    let is_open_vrp = false;
    let locked_ids = &[];
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, is_open_vrp);
    let mut insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(
            Arc::new(problem),
            (solution, None),
            create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
        ),
        locked_ids,
    );

    let jobs = extract_jobs(&mut insertion_ctx, route_idx, sequence_size);

    assert_eq!(jobs.len(), 3);
}
