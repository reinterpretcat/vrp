use super::{RandomRouteRemoval, Ruin};
use crate::construction::heuristics::InsertionContext;
use crate::helpers::models::domain::*;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition, Problem};
use crate::solver::search::{RemovalLimits, WorstRouteRemoval};
use std::sync::Arc;

#[test]
fn can_remove_whole_random_routes_from_context() {
    let limits = RemovalLimits { removed_activities_range: 10..10, affected_routes_range: 2..2 };
    let matrix = (4, 4);
    let ints = vec![10, 2, 0, 2];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, 1000., false);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, vec![1.]))),
    );

    let insertion_ctx = RandomRouteRemoval::new(limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(insertion_ctx.solution.required.len(), 8);
}

#[test]
fn can_remove_parts_random_routes_from_context() {
    let limits = RemovalLimits { removed_activities_range: 10..10, affected_routes_range: 1..1 };
    let matrix = (8, 1);
    let ints = vec![10, 1, 0, 2];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, 1000., false);
    let problem = Problem {
        jobs: problem.jobs.clone(),
        locks: vec![Arc::new(Lock {
            condition_fn: Arc::new(|_| false),
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
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, vec![]))),
    );

    let insertion_ctx = RandomRouteRemoval::new(limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(
        get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required),
        vec!["c1", "c2", "c4", "c5", "c6", "c7"]
    );
    assert_eq!(get_customer_ids_from_routes_sorted(&insertion_ctx), vec![vec!["c0", "c3"]]);
}

#[test]
fn can_remove_worst_route() {
    let limits = RemovalLimits { removed_activities_range: 10..10, affected_routes_range: 3..3 };
    let matrix = (4, 4);
    let ints = vec![3, 1];
    let reals = vec![1.];

    let (problem, mut solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, 1000., false);
    solution.routes[2].tour.remove_activity_at(1);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
    );

    let insertion_ctx = WorstRouteRemoval::new(limits)
        .run(&create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required), vec!["c10", "c11", "c9"]);
    assert_eq!(insertion_ctx.solution.routes.len(), 3);
}
