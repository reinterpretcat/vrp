use super::*;
use crate::helpers::algorithms::p;
use crate::helpers::models::domain::create_empty_problem;
use crate::helpers::solver::*;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Location;
use crate::utils::{DefaultRandom, Random};
use std::sync::Arc;

fn create_test_distances() -> Vec<f64> {
    generate_matrix_distances_from_points(&[
        p(0., 0.), // A
        p(5., 5.),
        p(0., 10.),
        p(5., 15.),
        p(20., 20.), // Ghost
        p(25., 0.),  // B
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
    case_00: ((8, 8), 1, |data: Vec<f64>| (data.clone(), data), 2.),

    case_01: ((8, 8), 3,  |data: Vec<f64>| (data.clone(), data), 2.),
    case_02: ((8, 8), 6,  |data: Vec<f64>| (data.clone(), data), 2.828),
    case_03: ((8, 8), 10, |data: Vec<f64>| (data.clone(), data), 4.472),
    case_04: ((8, 8), 18, |data: Vec<f64>| (data.clone(), data), 6.325),

    case_05:  ((8, 1), 3, |data: Vec<f64>| (data.clone(), data), 4.),
    case_06:  ((8, 1), 6, |data: Vec<f64>| (data.clone(), data), 6.),

    case_07:  ((8, 1), 2, |_: Vec<f64>| (vec![0.; 64], create_test_distances()), 11.18),
    case_08:  ((8, 1), 3, |_: Vec<f64>| (vec![0.; 64], create_test_distances()), 10.),
}

fn can_estimate_epsilon_impl(
    matrix: (usize, usize),
    nth_neighbor: usize,
    matrix_modify: fn(Vec<f64>) -> (Vec<f64>, Vec<f64>),
    expected: f64,
) {
    let (problem, _) = generate_matrix_routes(matrix.0, matrix.1, matrix_modify);

    assert_eq!((estimate_epsilon(&problem, nth_neighbor) * 1000.).round() / 1000., expected);
}

parameterized_test! {can_create_job_clusters, (param, expected), {
    can_create_job_clusters_impl(param, expected);
}}

can_create_job_clusters! {
    case_01: ((3, 10.00), &[vec![0, 1, 2, 3], vec![5, 6, 7]]),
    case_02: ((3, 12.00), &[vec![0, 1, 2, 3], vec![5, 6, 7]]),
    case_03: ((3, 14.14), &[vec![0, 1, 2, 3], vec![5, 6, 7]]),

    case_04: ((3, 14.15), &[vec![0, 1, 2, 3], vec![4, 5, 6, 7]]),
    case_05: ((3, 15.81), &[vec![0, 1, 2, 3], vec![4, 5, 6, 7]]),

    case_06: ((3, 15.82), &[vec![0, 1, 2, 3, 4, 5, 6, 7]]),
    case_07: ((3, 18.00), &[vec![0, 1, 2, 3, 4, 5, 6, 7]]),
}

fn can_create_job_clusters_impl(param: (usize, f64), expected: &[Vec<Location>]) {
    let (problem, _) = generate_matrix_routes(8, 1, |_| (vec![0.; 64], create_test_distances()));
    let random: Arc<dyn Random + Send + Sync> = Arc::new(FakeRandom::new(vec![0, 0], vec![param.1]));

    let clusters = create_job_clusters(&problem, &random, &[param])
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
fn can_create_ruin_cluster_with_proper_params() {
    let (problem, _) = generate_matrix_routes(8, 1, |_| (vec![0.; 64], create_test_distances()));
    let removal = ClusterRemoval::new(Arc::new(problem), 3..4, JobRemovalLimit::default());

    assert_eq!(removal.params.len(), 1);
    assert_eq!(removal.params[0].0, 3);
    assert_eq!(removal.params[0].1, 10.);
}

#[test]
fn can_handle_empty_problem() {
    let problem = create_empty_problem();

    let removal = ClusterRemoval::new(problem, 3..4, JobRemovalLimit::default());

    assert_eq!(removal.params.len(), 1);
}

parameterized_test! {can_ruin_jobs, (limit, cluster_size, expected), {
    can_ruin_jobs_impl(limit, cluster_size, expected);
}}

can_ruin_jobs! {
    case_01: (4, 3..4, 4),
    case_02: (5, 3..4, 5),
    case_03: (8, 3..4, 7),
}

fn can_ruin_jobs_impl(limit: usize, cluster_size: Range<usize>, expected: usize) {
    let limit = JobRemovalLimit::new(limit, limit, 1.);
    let (problem, solution) = generate_matrix_routes(8, 1, |_| (vec![0.; 64], create_test_distances()));
    let problem = Arc::new(problem);
    let insertion_ctx =
        InsertionContext::new_from_solution(problem.clone(), (solution, None), Arc::new(DefaultRandom::default()));

    let insertion_ctx = ClusterRemoval::new(problem, cluster_size, limit)
        .run(&mut create_default_refinement_ctx(insertion_ctx.problem.clone()), insertion_ctx);

    assert_eq!(insertion_ctx.solution.unassigned.len(), 0);
    assert_eq!(insertion_ctx.solution.locked.len(), 0);
    assert_eq!(insertion_ctx.solution.required.len(), expected);
    assert_eq!(
        insertion_ctx.solution.routes.iter().map(|route| route.route.tour.job_count()).sum::<usize>(),
        8 - expected
    );
}
