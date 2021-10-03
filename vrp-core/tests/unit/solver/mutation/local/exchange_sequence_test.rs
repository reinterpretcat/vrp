use super::*;
use crate::helpers::models::domain::*;
use crate::helpers::solver::*;
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use std::sync::Arc;

//let expected_ids = &[vec!["c0", "c1", "c2", "c3", "c4"], vec!["c5", "c6", "c7", "c8", "c9"]];

parameterized_test! { can_extract_jobs, (route_idx, start_idx, sequence_size, locked_ids, expected_route_ids, expected_extracted_ids), {
    can_extract_jobs_impl(route_idx, start_idx, sequence_size, locked_ids, expected_route_ids, expected_extracted_ids);
}}

can_extract_jobs! {
    case_01: (1, 1, 3, &[], &[vec!["c0", "c1", "c2", "c3", "c4"], vec!["c5", "c9"]], &[vec!["c6", "c7", "c8"]]),
    case_02: (1, 0, 2, &[], &[vec!["c0", "c1", "c2", "c3", "c4"], vec!["c7", "c8", "c9"]], &[vec!["c5", "c6"]]),
    case_03: (1, 0, 2, &["c6", "c7"], &[vec!["c0", "c1", "c2", "c3", "c4"], vec!["c6", "c7", "c9"]], &[vec!["c5", "c8"]]),
}

fn can_extract_jobs_impl(
    route_idx: usize,
    start_idx: i32,
    sequence_size: usize,
    locked_ids: &[&str],
    expected_route_ids: &[Vec<&str>],
    expected_extracted_ids: &[Vec<&str>],
) {
    let matrix = (5, 2);
    let is_open_vrp = false;
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, is_open_vrp);
    let mut insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(
            Arc::new(problem),
            (solution, None),
            create_test_environment_with_random(Arc::new(FakeRandom::new(vec![start_idx], vec![]))),
        ),
        locked_ids,
    );

    let jobs = extract_jobs(&mut insertion_ctx, route_idx, sequence_size);

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), expected_route_ids, "");
    compare_with_ignore(&[get_customer_ids_from_jobs(jobs.as_slice())], expected_extracted_ids, "")
}
