use super::*;
use crate::helpers::construction::clustering::dbscan::create_test_distances;
use crate::helpers::construction::clustering::p;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::{generate_matrix_distances_from_points, generate_matrix_routes};
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Location;
use crate::models::{Extras, GoalContext};
use crate::prelude::{ActivityCost, TransportCost};

type MatrixModFn = fn(Vec<Float>) -> (Vec<i32>, Vec<i32>);

#[test]
fn can_get_max_curvature() {
    let values =
        &[p(0., 0.), p(1., 0.25), p(2., 0.5), p(3., 0.75), p(4., 1.), p(6., 2.), p(7., 4.), p(8., 6.), p(9., 8.)];

    assert_eq!(get_max_curvature(values), 2.);
}

fn goal_factory(_: Arc<dyn TransportCost>, _: Arc<dyn ActivityCost>, _: &Extras) -> GoalContext {
    TestGoalContextBuilder::with_transport_feature().build()
}

fn scale_matrix_by_power_of_three(data: Vec<Float>) -> (Vec<i32>, Vec<i32>) {
    let data = data.into_iter().map(|i| (i * 1000.).round() as i32).collect::<Vec<_>>();
    (data.clone(), data)
}

parameterized_test! {can_estimate_epsilon, (matrix, nth_neighbor, matrix_modify, expected), {
    can_estimate_epsilon_impl(matrix, nth_neighbor, matrix_modify, expected);
}}

can_estimate_epsilon! {
    case_00: ((8, 8), 1, scale_matrix_by_power_of_three, 1000.),

    case_01: ((8, 8), 3,  scale_matrix_by_power_of_three, 1500.),
    case_02: ((8, 8), 6,  scale_matrix_by_power_of_three, 2237.),
    case_03: ((8, 8), 18, scale_matrix_by_power_of_three, 4079.),

    case_04:  ((8, 1), 3, scale_matrix_by_power_of_three, 2000.),
    case_05:  ((8, 1), 6, scale_matrix_by_power_of_three, 3714.),

    case_06:  ((8, 1), 2, |_: Vec<Float>| (vec![0; 64], create_test_distances(1000.)), 6084.),
    case_07:  ((8, 1), 3, |_: Vec<Float>| (vec![0; 64], create_test_distances(1000.)), 10419.),
}

fn can_estimate_epsilon_impl(matrix: (usize, usize), nth_neighbor: usize, matrix_modify: MatrixModFn, expected: Float) {
    let (problem, _) = generate_matrix_routes(
        matrix.0,
        matrix.1,
        false,
        goal_factory,
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        matrix_modify,
    );

    assert_eq!(estimate_epsilon(&problem, nth_neighbor).round(), expected.round());
}

parameterized_test! {can_estimate_epsilon_having_zero_costs, min_points, {
    can_estimate_epsilon_having_zero_costs_impl(min_points);
}}

can_estimate_epsilon_having_zero_costs! {
    case_01: 1,
    case_02: 2,
    case_03: 3,
    case_04: 4,
}

fn can_estimate_epsilon_having_zero_costs_impl(min_points: usize) {
    let (problem, _) = generate_matrix_routes(
        8,
        1,
        false,
        goal_factory,
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |_| {
            let distances = generate_matrix_distances_from_points(
                &[p(0., 0.), p(0., 0.), p(0., 0.), p(0., 0.), p(5., 0.), p(10., 0.), p(20., 0.), p(30., 0.)],
                1.,
            );
            (vec![0; 64], distances)
        },
    );

    let costs = get_average_costs(&problem, min_points);

    assert!(!costs.is_empty());
}

parameterized_test! {can_create_job_clusters, (param, expected), {
    can_create_job_clusters_impl(param, expected);
}}

can_create_job_clusters! {
    case_01: ((2, 8.00), &[vec![0, 1, 2, 3], vec![5, 6, 7]]),
    case_02: ((3, 12.00), &[vec![0, 1, 2, 3]]),
    case_03: ((3, 25.00), &[vec![0, 1, 2, 3, 5, 6, 7]]),
}

fn can_create_job_clusters_impl(param: (usize, Float), expected: &[Vec<Location>]) {
    let (min_points, epsilon) = param;
    let (problem, _) = generate_matrix_routes(
        8,
        1,
        false,
        goal_factory,
        |id, location| TestSingleBuilder::default().id(id).location(location).build_shared(),
        |v| v,
        |_| (vec![0; 64], create_test_distances(1.)),
    );
    let random: Arc<dyn Random> = Arc::new(FakeRandom::new(vec![0, 0], vec![epsilon]));

    let clusters = create_job_clusters(&problem, random.as_ref(), Some(min_points), Some(epsilon))
        .iter()
        .map(|cluster| {
            let mut cluster =
                cluster.iter().map(|job| job.as_single().unwrap().places[0].location.unwrap()).collect::<Vec<_>>();
            cluster.sort_unstable();
            cluster
        })
        .collect::<Vec<_>>();

    assert_eq!(clusters, expected);
}
