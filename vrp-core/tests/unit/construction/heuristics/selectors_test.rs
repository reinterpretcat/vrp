use super::*;
use crate::helpers::models::domain::create_empty_insertion_context;
use crate::helpers::models::problem::test_single_with_id;
use crate::helpers::models::solution::create_empty_route_ctx;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Cost;
use std::sync::Arc;

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

fn can_compare_insertion_result_with_noise_impl(
    left: InsertionResult,
    right: InsertionResult,
    reals: Vec<f64>,
    expected_result: Option<f64>,
) {
    let noise_probability = 0.1;
    let noise_range = (0.9, 1.2);
    let random = Arc::new(FakeRandom::new(vec![], reals));
    let noise = Noise::new(noise_probability, noise_range, random);

    let actual_result =
        NoiseResultSelector::new(noise).select_insertion(&create_empty_insertion_context(), left, right);

    match (actual_result, expected_result) {
        (InsertionResult::Success(success), Some(cost)) => assert_eq!(success.cost, cost),
        (InsertionResult::Failure(_), None) => {}
        _ => unreachable!(),
    }
}

mod iterators {
    use crate::helpers::solver::generate_matrix_routes_with_defaults;

    #[test]
    fn can_get_size_hint_for_tour_legs() {
        let (_, solution) = generate_matrix_routes_with_defaults(5, 1, false);

        assert_eq!(solution.routes[0].tour.legs().skip(2).size_hint().0, 4);
    }
}
