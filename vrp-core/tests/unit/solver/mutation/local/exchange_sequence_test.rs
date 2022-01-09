use super::*;
use crate::helpers::models::domain::*;
use crate::helpers::solver::*;
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use rosomaxa::prelude::Environment;
use std::sync::Arc;

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
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 2, false);
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

parameterized_test! { can_insert_jobs, (start_idx, insert_job_ids, disallowed_pairs, reverse_probability, expected_route_ids, expected_unassigned_ids), {
    can_insert_jobs_impl(start_idx, insert_job_ids, disallowed_pairs, reverse_probability, expected_route_ids, &[expected_unassigned_ids]);
}}

can_insert_jobs! {
    case_01: (0, &["c5", "c6"], vec![], 1., &[vec!["c5", "c6", "c0", "c1", "c2", "c3", "c4"]], vec![]),
    case_02: (1, &["c5", "c6"], vec![], 1., &[vec!["c0", "c5", "c6", "c1", "c2", "c3", "c4"]], vec![]),
    case_03: (5, &["c5", "c6"], vec![], 1., &[vec!["c0", "c1", "c2", "c3", "c4", "c5", "c6"]], vec![]),
    case_04: (0, &["c5", "c6"], vec![], 0., &[vec!["c6", "c5", "c0", "c1", "c2", "c3", "c4"]], vec![]),

    case_05: (2, &["c5", "c6"], vec![("c1", "c2")], 1., &[vec!["c0", "c1", "c2", "c5", "c6", "c3", "c4"]], vec![]),
    case_06: (1, &["c5", "c6"], vec![("c5", "c1")], 1., &[vec!["c0", "c5", "c1", "c6", "c2", "c3", "c4"]], vec![]),
    case_07: (5, &["c5", "c6"], vec![("c5", "cX")], 1., &[vec!["c0", "c1", "c2", "c3", "c4", "c5"]], vec!["c6"]),
}

fn can_insert_jobs_impl(
    start_idx: i32,
    insert_job_ids: &[&str],
    disallowed_pairs: Vec<(&str, &str)>,
    reverse_probability: f64,
    expected_route_ids: &[Vec<&str>],
    expected_unassigned_ids: &[Vec<&str>],
) {
    let route_idx = 0;
    let reverse_probability_threshold = 0.01;

    let (mut problem, solution) = generate_matrix_routes_with_defaults(5, 2, false);
    add_leg_constraint(&mut problem, disallowed_pairs);
    let mut insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(
            vec![start_idx],
            vec![reverse_probability, 1., reverse_probability, 1.],
        ))),
    );
    let jobs = get_jobs_by_ids(&insertion_ctx, insert_job_ids);

    insert_jobs(&mut insertion_ctx, route_idx, jobs, reverse_probability_threshold, 0.);

    compare_with_ignore(
        vec![get_customer_ids_from_routes(&insertion_ctx).get(0).cloned().unwrap()].as_slice(),
        expected_route_ids,
        "",
    );
    compare_with_ignore(vec![get_customer_ids_from_unassigned(&insertion_ctx)].as_slice(), expected_unassigned_ids, "");
}

parameterized_test! { can_exchange_jobs, (first_route, second_route, expected_route_ids), {
    can_exchange_jobs_impl(first_route, second_route, expected_route_ids);
}}

can_exchange_jobs! {
    case_01: ((0, 3, 1, 2), (1, 2, 2, 1), &[vec!["c0", "c4", "c7", "c8"], vec!["c5", "c1", "c2", "c3", "c6", "c9"]]),
    case_02: ((0, 5, 0, 0), (0, 0, -1, -1), &[vec!["c0", "c1", "c2", "c3", "c4"], vec!["c5", "c6", "c7", "c8", "c9"]]),
    case_03: ((0, 3, 1, 2), (0, 2, -1, -1), &[vec!["c0", "c4", "c1", "c2", "c3"], vec!["c5", "c6", "c7", "c8", "c9"]]),
}

fn can_exchange_jobs_impl(
    first_route: (i32, i32, i32, i32),
    second_route: (i32, i32, i32, i32),
    expected_route_ids: &[Vec<&str>],
) {
    let (first_route_index, first_sequence_size, first_start_index, first_insertion_idx) = first_route;
    let (second_route_index, second_sequence_size, second_start_index, second_insertion_idx) = second_route;
    let ints = vec![
        first_route_index,
        first_sequence_size,
        first_start_index,
        second_route_index,
        second_sequence_size,
        second_start_index,
        first_insertion_idx,
        second_insertion_idx,
    ];
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 2, false);
    let mut insertion_ctx = InsertionContext::new_from_solution(
        Arc::new(problem),
        (solution, None),
        create_test_environment_with_random(Arc::new(FakeRandom::new(ints, vec![1., 1., 1., 1.]))),
    );

    exchange_jobs(&mut insertion_ctx, &[0, 1], 4, 0.01, 0.01);

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), expected_route_ids, "");
}

parameterized_test! { can_get_route_indices, (rows, locked_ids, expected), {
    can_get_route_indices_impl(rows, locked_ids, expected);
}}

can_get_route_indices! {
    case_01: (5, &[], &[0, 1]),
    case_02: (1, &[], &[]),
    case_03: (5, &["c1", "c2", "c3", "c4"], &[1]),
}

fn can_get_route_indices_impl(rows: usize, locked_ids: &[&str], expected: &[usize]) {
    let (problem, solution) = generate_matrix_routes_with_defaults(rows, 2, false);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), Arc::new(Environment::default())),
        locked_ids,
    );

    let indices = get_route_indices(&insertion_ctx);

    assert_eq!(indices, expected);
}

parameterized_test! { can_exchange_sequence, (locked_ids, expected), {
    can_exchange_sequence_impl(locked_ids, expected);
}}

can_exchange_sequence! {
    case_01: (&[], Some(())),
    case_02: (&["c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8"], None),
}

fn can_exchange_sequence_impl(locked_ids: &[&str], expected: Option<()>) {
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 2, false);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), Arc::new(Environment::default())),
        locked_ids,
    );
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = ExchangeSequence::default().explore(&refinement_ctx, &insertion_ctx);

    assert_eq!(result.map(|_| ()), expected);
}
