use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::*;
use crate::models::common::{Schedule, TimeWindow};
use crate::models::solution::*;
use crate::utils::Environment;

fn create_insertion_success(insertion_ctx: &InsertionContext, insertion_data: (usize, &str, usize)) -> InsertionResult {
    let (route_idx, job_id, insertion_idx) = insertion_data;

    let context = insertion_ctx.solution.routes.get(route_idx).cloned().unwrap();
    let job = get_jobs_by_ids(&insertion_ctx, &[job_id]).first().cloned().unwrap();
    let activity = Activity {
        place: Place { location: 0, duration: 0.0, time: TimeWindow::new(0., 1.) },
        schedule: Schedule { arrival: 0., departure: 0. },
        job: Some(job.to_single().clone()),
        commute: None,
    };

    InsertionResult::Success(InsertionSuccess { cost: 0., job, activities: vec![(activity, insertion_idx)], context })
}

fn create_insertion_ctx(matrix: (usize, usize), disallowed_pairs: Vec<(&str, &str)>) -> InsertionContext {
    let (mut problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, false);
    let environment = Arc::new(Environment::default());
    add_leg_constraint(&mut problem, disallowed_pairs);

    InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment)
}

fn create_default_selectors() -> (VariableLegSelector, BestResultSelector) {
    let leg_selector = VariableLegSelector::new(Environment::default().random);
    let result_selector = BestResultSelector::default();

    (leg_selector, result_selector)
}

#[test]
fn can_use_exchange_swap_star() {
    let locked_ids = &[];

    let matrix = (3, 3);
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone()),
        locked_ids,
    );

    let insertion_ctx = ExchangeSwapStar::new(environment.random.clone())
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    compare_with_ignore(
        get_customer_ids_from_routes(&insertion_ctx).as_slice(),
        &[vec!["c0", "c1", "c6"], vec!["c2", "c3", "c5"], vec!["c4", "c7", "c8"]],
        "",
    );
}

#[test]
fn can_exchange_jobs_in_routes() {
    let route_pair = (0, 1);
    let disallowed_pairs = vec![];
    let expected_route_ids = vec![vec!["c0", "c1", "c3"], vec!["c4", "c5", "c2"]];

    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs);
    let (leg_selector, result_selector) = create_default_selectors();

    try_exchange_jobs_in_routes(&mut insertion_ctx, route_pair, &leg_selector, &result_selector);

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), &expected_route_ids, "");
}

parameterized_test! { can_exchange_single_jobs, (outer_insertion, inner_insertion, disallowed_pairs, expected_route_ids), {
    can_exchange_single_jobs_impl(outer_insertion, inner_insertion, disallowed_pairs, expected_route_ids);
}}

can_exchange_single_jobs! {
    case_01: ((0, "c3", 0), (1, "c1", 0), vec![], vec![vec!["c3", "c0", "c2"], vec!["c1", "c4", "c5"]]),
    case_02: ((0, "c3", 1), (1, "c1", 0), vec![], vec![vec!["c0", "c3", "c2"], vec!["c1", "c4", "c5"]]),
    case_03: ((0, "c3", 2), (1, "c1", 0), vec![], vec![vec!["c0", "c3", "c2"], vec!["c1", "c4", "c5"]]),
    case_04: ((0, "c3", 3), (1, "c1", 0), vec![], vec![vec!["c0", "c2", "c3"], vec!["c1", "c4", "c5"]]),

    case_05: ((0, "c3", 0), (1, "c1", 0), vec![], vec![vec!["c3", "c0", "c2"], vec!["c1", "c4", "c5"]]),
    case_06: ((0, "c3", 0), (1, "c1", 1), vec![], vec![vec!["c3", "c0", "c2"], vec!["c1", "c4", "c5"]]),
    case_07: ((0, "c3", 0), (1, "c1", 2), vec![], vec![vec!["c3", "c0", "c2"], vec!["c4", "c1", "c5"]]),
    case_08: ((0, "c3", 0), (1, "c1", 3), vec![], vec![vec!["c3", "c0", "c2"], vec!["c4", "c5", "c1"]]),

    case_09: ((0, "c3", 0), (1, "c1", 0), vec![("cX", "c4")], vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"]]),
}

fn can_exchange_single_jobs_impl(
    outer_insertion: (usize, &str, usize),
    inner_insertion: (usize, &str, usize),
    disallowed_pairs: Vec<(&str, &str)>,
    expected_route_ids: Vec<Vec<&str>>,
) {
    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs);
    let (leg_selector, result_selector) = create_default_selectors();
    let insertion_pair = (
        create_insertion_success(&insertion_ctx, outer_insertion),
        create_insertion_success(&insertion_ctx, inner_insertion),
    );

    try_exchange_jobs(&mut insertion_ctx, insertion_pair, &leg_selector, &result_selector);

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), &expected_route_ids, "");
}

parameterized_test! { can_find_insertion_cost, (job_id, expected), {
    can_find_insertion_cost_impl(job_id, expected);
}}

can_find_insertion_cost! {
    case_01: ("c0", 0.),
    case_02: ("c1", 0.),
    case_03: ("c2", 4.),
}

fn can_find_insertion_cost_impl(job_id: &str, expected: Cost) {
    let matrix = (3, 1);
    let insertion_ctx = create_insertion_ctx(matrix, vec![]);
    let (leg_selector, result_selector) = create_default_selectors();
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selector, &result_selector);
    let job = get_jobs_by_ids(&insertion_ctx, &[job_id]).first().cloned().unwrap();
    let route_ctx = insertion_ctx.solution.routes.first().unwrap();

    let result = find_insertion_cost(&search_ctx, &job, route_ctx);

    assert_eq!(result, expected);
}
