use super::{RandomRouteRemoval, Ruin};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::*;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::random::FakeRandom;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition, Problem};
use std::sync::Arc;

#[test]
fn can_remove_whole_routes_from_context() {
    let params = (1usize, 3usize, 1.);
    let matrix = (4, 4);
    let ints = vec![2, 0, 2];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, false);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        Arc::new(FakeRandom::new(ints, vec![])),
    );

    let insertion_ctx = RandomRouteRemoval::new(params.0, params.1, params.2)
        .run(&mut create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(insertion_ctx.solution.required.len(), 8);
}

#[test]
fn can_remove_parts_routes_from_context() {
    let params = (1usize, 3usize, 1.);
    let matrix = (8, 1);
    let ints = vec![2, 0, 2];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, false);
    let problem = Problem {
        jobs: problem.jobs.clone(),
        locks: vec![Arc::new(Lock {
            condition: Arc::new(|_| false),
            details: vec![LockDetail {
                order: LockOrder::Any,
                position: LockPosition::Any,
                jobs: problem.jobs.all().filter(|job| ["c0", "c3"].contains(&get_customer_id(job).as_str())).collect(),
            }],
            is_lazy: false,
        })],
        ..problem
    };
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        Arc::new(FakeRandom::new(ints, vec![])),
    );

    let insertion_ctx = RandomRouteRemoval::new(params.0, params.1, params.2)
        .run(&mut create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(
        get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required),
        vec!["c1", "c2", "c4", "c5", "c6", "c7"]
    );
    assert_eq!(get_customer_ids_from_routes_sorted(&insertion_ctx), vec![vec!["c0", "c3"]]);
}
