use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes_with_defaults};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::IdDimension;
use std::sync::Arc;

fn compare_ids_with_ignore(left: Vec<Vec<String>>, right: Vec<Vec<&str>>) {
    if left.len() != right.len() {
        assert_eq!(left, right);
    }

    left.iter().zip(right.iter()).for_each(|(a_vec, b_vec)| {
        if a_vec.len() != b_vec.len() {
            assert_eq!(left, right);
        }

        a_vec.iter().zip(b_vec.iter()).for_each(|(a_value, b_value)| {
            if a_value != "cX" && *b_value != "cX" && a_value != b_value {
                assert_eq!(left, right);
            }
        });
    })
}

fn extend_with_locked(mut ctx: InsertionContext, job_ids: &[&str]) -> InsertionContext {
    let ids = ctx.problem.jobs.all().filter(|job| job_ids.contains(&job.dimens().get_id().unwrap().as_str()));
    ctx.solution.locked.extend(ids);

    ctx
}

parameterized_test! {can_use_exchange_inter_route_best_operator, (seed_route, seed_job, locked_ids, expected_ids), {
    can_use_exchange_inter_route_best_operator_impl(seed_route, seed_job, locked_ids, expected_ids);
}}

can_use_exchange_inter_route_best_operator! {
    case_01: (0, 2, &[], vec![vec!["c0", "c2", "c3"], vec!["c1", "c4", "c5"], vec!["c6", "c7", "c8"]]),
    case_02: (2, 3, &[], vec![vec!["c0", "c1", "c2"], vec!["cX", "cX", "c8"], vec!["cX", "c6", "c7"]]),
    case_03: (1, 2, &[], vec![vec!["c0", "c1", "c2"], vec!["c3", "c5", "c6"], vec!["c4", "c7", "c8"]]),
    case_04: (1, 3, &[], vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c6"], vec!["c5", "c7", "c8"]]),

    case_05: (2, 3, &["c3", "c4"], vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c8"], vec!["c5", "c6", "c7"]]),
    case_06: (2, 1, &["c4", "c5"], vec![vec!["c0", "c1", "c2"], vec!["c4", "c5", "c6"], vec!["c3", "c7", "c8"]]),
    case_07: (1, 1, &["c0", "c1", "c2"], vec![vec!["c0", "c1", "c2"], vec!["c4", "c5", "c6"], vec!["c3", "c7", "c8"]]),
}

fn can_use_exchange_inter_route_best_operator_impl(
    seed_route: i32,
    seed_job: i32,
    locked_ids: &[&str],
    expected_ids: Vec<Vec<&str>>,
) {
    let matrix = (3, 3);
    let ints = vec![seed_route, seed_job];
    let reals = vec![1.; 128];

    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = extend_with_locked(
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

    compare_ids_with_ignore(get_customer_ids_from_routes(&new_insertion_ctx), expected_ids);
}
