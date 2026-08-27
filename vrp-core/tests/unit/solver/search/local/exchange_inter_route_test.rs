use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::*;
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use std::sync::Arc;

parameterized_test! {can_use_exchange_inter_route_best_operator, (seed_route, seed_job, locked_ids, expected_ids), {
    can_use_exchange_inter_route_best_operator_impl(seed_route, seed_job, locked_ids, expected_ids);
}}

can_use_exchange_inter_route_best_operator! {
    case_01: (0, 2, &[], &[vec!["c0", "c2", "c3"], vec!["c1", "c4", "c5"], vec!["c6", "c7", "c8"]]),
    case_02: (2, 3, &[], &[vec!["c0", "c1", "c2"], vec!["cX", "cX", "c8"], vec!["cX", "c6", "c7"]]),
    case_03: (1, 2, &[], &[vec!["c0", "c1", "c2"], vec!["c3", "c5", "c6"], vec!["c4", "c7", "c8"]]),
    case_04: (1, 3, &[], &[vec!["c0", "c1", "c2"], vec!["c3", "c4", "c6"], vec!["c5", "c7", "c8"]]),

    case_05: (2, 3, &["c3", "c4"], &[vec!["c0", "c1", "c2"], vec!["c3", "c4", "c8"], vec!["c5", "c6", "c7"]]),
    case_06: (2, 1, &["c4", "c5"], &[vec!["c0", "c1", "c2"], vec!["c4", "c5", "c6"], vec!["c3", "c7", "c8"]]),
    case_07: (1, 1, &["c0", "c1", "c2"], &[vec!["c0", "c1", "c2"], vec!["c4", "c5", "c6"], vec!["c3", "c7", "c8"]]),
}

fn can_use_exchange_inter_route_best_operator_impl(
    seed_route: i32,
    seed_job: i32,
    locked_ids: &[&str],
    expected_ids: &[Vec<&str>],
) {
    let matrix = (3, 3);
    let ints = [seed_route, seed_job].into_iter().chain([16; 1024]).collect();
    let reals = vec![1.; 1024];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(
            Arc::new(problem),
            (solution, None),
            create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
        ),
        locked_ids,
    );

    let new_insertion_ctx = ExchangeInterRouteBest::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    compare_with_ignore(get_customer_ids_from_routes(&new_insertion_ctx).as_slice(), expected_ids, "cX");
}

#[test]
fn can_skip_locked_seed_job() {
    let ints = [0, 1].into_iter().chain([16; 1024]).collect();
    let reals = vec![1.; 1024];
    let (problem, solution) = generate_matrix_routes_with_defaults(3, 3, true);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(
            Arc::new(problem),
            (solution, None),
            create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
        ),
        &["c0"],
    );

    let new_insertion_ctx = ExchangeInterRouteBest::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    assert!(get_customer_ids_from_routes(&new_insertion_ctx)[0].iter().any(|job_id| job_id == "c0"));
}

#[test]
fn can_search_many_routes_in_one_reduction() {
    let ints = [0, 1].into_iter().chain([16; 4096]).collect();
    let reals = vec![1.; 4096];
    let (problem, solution) = generate_matrix_routes_with_defaults(2, 17, true);
    let insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, reals))),
    );

    let new_insertion_ctx = ExchangeInterRouteBest::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    assert_eq!(new_insertion_ctx.solution.routes.len(), 17);
    assert_eq!(new_insertion_ctx.solution.get_jobs_amount(), insertion_ctx.solution.get_jobs_amount());
}
