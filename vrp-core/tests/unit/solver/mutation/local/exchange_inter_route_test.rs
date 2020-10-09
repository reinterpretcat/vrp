use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::models::problem::test_single_with_id;
use crate::helpers::models::solution::create_empty_route_ctx;
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes};
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::{Cost, IdDimension};
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

    let (problem, solution) = generate_matrix_routes(matrix.0, matrix.1, true, |data| (data.clone(), data));
    let insertion_ctx = extend_with_locked(
        InsertionContext::new_from_solution(
            Arc::new(problem),
            (solution, None),
            Arc::new(FakeRandom::new(ints, reals)),
        ),
        locked_ids,
    );

    let new_insertion_ctx = ExchangeInterRouteBest::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    compare_ids_with_ignore(get_customer_ids_from_routes(&new_insertion_ctx), expected_ids);
}

fn make_success(cost: Cost) -> InsertionResult {
    InsertionResult::make_success(cost, Job::Single(test_single_with_id("job1")), vec![], create_empty_route_ctx())
}


parameterized_test! {can_compare_insertion_result_with_noise, (left, right, reals, expected_result), {
    can_compare_insertion_result_with_noise_impl(left, right, reals, expected_result);
}}

can_compare_insertion_result_with_noise! {
    case01: (make_success(10.), make_success(11.), vec![0.05, 1.2, 0.05, 1.],  Some(11.)),
    case02: (make_success(11.), make_success(10.), vec![0.05, 0.8, 0.05, 1.],  Some(11.)),
    case03: (make_success(11.), make_success(10.), vec![0.05, 1., 0.2],  Some(10.)),

    case04: (InsertionResult::make_failure(), make_success(11.), vec![],  Some(11.)),
    case05: (make_success(10.), InsertionResult::make_failure(), vec![],  Some(10.)),
    case06: (InsertionResult::make_failure(), InsertionResult::make_failure(), vec![],  None),
}

fn can_compare_insertion_result_with_noise_impl(left: InsertionResult, right: InsertionResult, reals: Vec<f64>, expected_result: Option<f64>) {
    let noise_probability = 0.1;
    let noise_range = (0.9, 1.2);
    let random = Arc::new(FakeRandom::new(vec![], reals));
    let noise = Noise::new(noise_probability, noise_range, random);

    let actual_result = compare_insertion_result_with_noise(left, right, &noise);

    match (actual_result, expected_result) {
        (InsertionResult::Success(success), Some(cost)) => assert_eq!(success.cost, cost),
        (InsertionResult::Failure(_), None) => {}
        _ => unreachable!()
    }
}
