use super::*;
use crate::helpers::algorithms::p;
use crate::helpers::models::domain::create_empty_problem;
use crate::helpers::models::problem::test_single_with_id_and_location;
use crate::helpers::solver::*;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Location;
use crate::utils::{Environment, Random};
use std::sync::Arc;

fn create_test_distances() -> Vec<f64> {
    generate_matrix_distances_from_points(&[
        p(0., 0.), // A
        p(5., 5.),
        p(0., 10.),
        p(5., 15.),
        p(200., 200.), // Ghost
        p(25., 0.),    // B
        p(30., 5.),
        p(30., 10.),
    ])
}

#[test]
fn can_get_max_curvature() {
    let values =
        &[p(0., 0.), p(1., 0.25), p(2., 0.5), p(3., 0.75), p(4., 1.), p(6., 2.), p(7., 4.), p(8., 6.), p(9., 8.)];

    assert_eq!(get_max_curvature(values), 2.);
}

parameterized_test! {can_estimate_epsilon, (matrix, nth_neighbor, matrix_modify, expected), {
    can_estimate_epsilon_impl(matrix, nth_neighbor, matrix_modify, expected);
}}

can_estimate_epsilon! {
    case_00: ((8, 8), 1, |data: Vec<f64>| (data.clone(), data), 1.),

    case_01: ((8, 8), 3,  |data: Vec<f64>| (data.clone(), data), 1.5),
    case_02: ((8, 8), 6,  |data: Vec<f64>| (data.clone(), data), 2.237),
    case_03: ((8, 8), 18, |data: Vec<f64>| (data.clone(), data), 4.079),

    case_04:  ((8, 1), 3, |data: Vec<f64>| (data.clone(), data), 2.0),
    case_05:  ((8, 1), 6, |data: Vec<f64>| (data.clone(), data), 4.571),

    case_06:  ((8, 1), 2, |_: Vec<f64>| (vec![0.; 64], create_test_distances()), 6.084),
    case_07:  ((8, 1), 3, |_: Vec<f64>| (vec![0.; 64], create_test_distances()), 10.419),
}

fn can_estimate_epsilon_impl(
    matrix: (usize, usize),
    nth_neighbor: usize,
    matrix_modify: fn(Vec<f64>) -> (Vec<f64>, Vec<f64>),
    expected: f64,
) {
    let (problem, _) = generate_matrix_routes(
        matrix.0,
        matrix.1,
        false,
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        matrix_modify,
    );

    assert_eq!((estimate_epsilon(&problem, nth_neighbor) * 1000.).round() / 1000., expected);
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
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        |_| {
            let distances = generate_matrix_distances_from_points(&[
                p(0., 0.),
                p(0., 0.),
                p(0., 0.),
                p(0., 0.),
                p(5., 0.),
                p(10., 0.),
                p(20., 0.),
                p(30., 0.),
            ]);
            (vec![0.; 64], distances)
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

fn can_create_job_clusters_impl(param: (usize, f64), expected: &[Vec<Location>]) {
    let (min_items, epsilon) = param;
    let (problem, _) = generate_matrix_routes(
        8,
        1,
        false,
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        |_| (vec![0.; 64], create_test_distances()),
    );
    let random: Arc<dyn Random + Send + Sync> = Arc::new(FakeRandom::new(vec![0, 0], vec![epsilon]));

    let clusters = create_job_clusters(&problem, random.as_ref(), min_items, epsilon)
        .iter()
        .map(|cluster| {
            let mut cluster =
                cluster.iter().map(|job| job.as_single().unwrap().places[0].location.unwrap()).collect::<Vec<_>>();
            cluster.sort();
            cluster
        })
        .collect::<Vec<_>>();

    assert_eq!(clusters, expected);
}

#[test]
fn can_create_ruin_cluster_with_default_params() {
    let environment = Arc::new(Environment::default());
    let (problem, _) = generate_matrix_routes(
        8,
        1,
        false,
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        |_| (vec![0.; 64], create_test_distances()),
    );

    let removal = ClusterRemoval::new_with_defaults(Arc::new(problem), environment);

    assert!(!removal.clusters.is_empty());
}

#[test]
fn can_handle_empty_problem() {
    let problem = create_empty_problem();

    let removal = ClusterRemoval::new(problem, Arc::new(Environment::default()), 3, RuinLimits::default());

    assert!(removal.clusters.is_empty());
}

parameterized_test! {can_ruin_jobs, (limit, cluster_size, expected), {
    can_ruin_jobs_impl(limit, cluster_size, expected);
}}

can_ruin_jobs! {
    case_01: (4, 3, 4),
    case_02: (5, 3, 4),
    case_03: (8, 3, 4),
}

fn can_ruin_jobs_impl(limit: usize, min_items: usize, expected: usize) {
    let limit = RuinLimits::new(limit, limit, 1., 8);
    let (problem, solution) = generate_matrix_routes(
        8,
        1,
        false,
        |id, location| test_single_with_id_and_location(id, location),
        |v| v,
        |_| (vec![0.; 64], create_test_distances()),
    );
    let problem = Arc::new(problem);
    let environment = Arc::new(Environment::default());
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());

    let insertion_ctx = ClusterRemoval::new(problem, environment, min_items, limit)
        .run(&mut create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), 0);
    assert_eq!(insertion_ctx.solution.locked.len(), 0);
    assert_eq!(insertion_ctx.solution.required.len(), expected);
    assert_eq!(
        insertion_ctx.solution.routes.iter().map(|route| route.route.tour.job_count()).sum::<usize>(),
        8 - expected
    );
}
