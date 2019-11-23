use crate::construction::states::InsertionContext;
use crate::helpers::models::domain::*;
use crate::helpers::refinement::generate_matrix_routes;
use crate::helpers::utils::random::FakeRandom;
use crate::models::{Lock, LockDetail, LockOrder, LockPosition, Problem};
use crate::refinement::ruin::random_route_removal::RandomRouteRemoval;
use crate::refinement::ruin::Ruin;
use crate::refinement::RefinementContext;
use std::sync::Arc;

#[test]
fn can_remove_whole_routes_from_context() {
    let params = (1usize, 3usize, 1.);
    let matrix = (4, 4);
    let ints = vec![2, 0, 2];

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (Arc::new(solution), None),
        Arc::new(FakeRandom::new(ints, vec![])),
    );

    let insertion_ctx = RandomRouteRemoval::new(params.0, params.1, params.2).run(
        &RefinementContext { problem: insertion_ctx.problem.clone(), population: vec![], generation: 0 },
        insertion_ctx,
    );

    assert_eq!(insertion_ctx.solution.required.len(), 8);
}

#[test]
fn can_remove_parts_routes_from_context() {
    let params = (1usize, 3usize, 1.);
    let matrix = (8, 1);
    let ints = vec![2, 0, 2];

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1);
    let problem = Problem {
        fleet: problem.fleet,
        jobs: problem.jobs.clone(),
        locks: vec![Arc::new(Lock {
            condition: Arc::new(|_| false),
            details: vec![LockDetail {
                order: LockOrder::Any,
                position: LockPosition::Any,
                jobs: problem.jobs.all().filter(|job| ["c0", "c3"].contains(&get_customer_id(job).as_str())).collect(),
            }],
        })],
        constraint: problem.constraint,
        activity: problem.activity,
        transport: problem.transport,
        objective: problem.objective,
        extras: problem.extras,
    };
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (Arc::new(solution), None),
        Arc::new(FakeRandom::new(ints, vec![])),
    );

    let insertion_ctx = RandomRouteRemoval::new(params.0, params.1, params.2).run(
        &RefinementContext { problem: insertion_ctx.problem.clone(), population: vec![], generation: 0 },
        insertion_ctx,
    );

    assert_eq!(
        get_sorted_customer_ids_from_jobs(&insertion_ctx.solution.required),
        vec!["c1", "c2", "c4", "c5", "c6", "c7"]
    );
    assert_eq!(get_customer_ids_from_routes_sorted(&insertion_ctx), vec![vec!["c0", "c3"]]);
}
