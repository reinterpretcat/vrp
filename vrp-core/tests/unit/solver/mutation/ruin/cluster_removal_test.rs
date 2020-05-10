use crate::helpers::algorithms::p;
use crate::helpers::models::problem::{MultiBuilder, SingleBuilder};
use crate::helpers::solver::generate_matrix_routes;
use crate::models::problem::Jobs;
use crate::solver::mutation::ruin::cluster_removal::{estimate_epsilon, get_max_curvature};
use std::sync::Arc;

#[test]
fn can_get_max_curvature() {
    let values =
        &[p(0., 0.), p(1., 0.25), p(2., 0.5), p(3., 0.75), p(4., 1.), p(6., 2.), p(7., 4.), p(8., 6.), p(9., 8.)];

    assert_eq!(get_max_curvature(values), 2.);
}

parameterized_test! {can_estimate_epsilon, (matrix, nth_neighbor, expected), {
    can_estimate_epsilon_impl(matrix, nth_neighbor, expected);
}}

can_estimate_epsilon! {
    case_00: ((8, 8), 1, 2.),

    case_01: ((8, 8), 3, 2.),
    case_02: ((8, 8), 6, 2.828),
    case_03: ((8, 8), 10, 4.472),
    case_04: ((8, 8), 18, 6.325),

    case_05:  ((8, 1), 3, 4.),
    case_06:  ((8, 1), 6, 8.),
}

fn can_estimate_epsilon_impl(matrix: (usize, usize), nth_neighbor: usize, expected: f64) {
    let (problem, _) = generate_matrix_routes(matrix.0, matrix.1, |data| (data.clone(), data));

    assert_eq!((estimate_epsilon(&problem, nth_neighbor) * 1000.).round() / 1000., expected);
}
