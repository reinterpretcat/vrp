use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::models::problem::get_vehicle_id;
use crate::helpers::solver::*;
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::{Schedule, TimeWindow};
use crate::models::solution::*;
use rosomaxa::prelude::Environment;

fn create_insertion_success(insertion_ctx: &InsertionContext, insertion_data: (usize, &str, usize)) -> InsertionResult {
    let (route_idx, job_id, insertion_idx) = insertion_data;

    let context = insertion_ctx.solution.routes.get(route_idx).cloned().unwrap();
    let job = get_jobs_by_ids(insertion_ctx, &[job_id]).first().cloned().unwrap();
    let activity = Activity {
        place: Place { location: 0, duration: 0.0, time: TimeWindow::new(0., 1.) },
        schedule: Schedule { arrival: 0., departure: 0. },
        job: Some(job.to_single().clone()),
        commute: None,
    };

    InsertionResult::Success(InsertionSuccess { cost: 0., job, activities: vec![(activity, insertion_idx)], context })
}

fn create_insertion_ctx(
    matrix: (usize, usize),
    disallowed_pairs: Vec<(&str, &str)>,
    is_open_vrp: bool,
) -> InsertionContext {
    let (problem, solution) =
        generate_matrix_routes_with_disallow_list(matrix.0, matrix.1, is_open_vrp, disallowed_pairs);
    let environment = Arc::new(Environment::default());

    InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment)
}

fn create_default_selectors() -> (VariableLegSelector, BestResultSelector) {
    let leg_selector = VariableLegSelector::new(Environment::default().random);
    let result_selector = BestResultSelector::default();

    (leg_selector, result_selector)
}

parameterized_test! { can_use_exchange_swap_star, (jobs_order, expected), {
    can_use_exchange_swap_star_impl(jobs_order, expected);
}}

can_use_exchange_swap_star! {
    case_01: (
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
    ),
    case_02: (
        vec![vec!["c0", "c1", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c8"]],
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
    ),
    case_03: (
        vec![vec!["c0", "c8", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c1"]],
        vec![vec!["c0", "c1", "c2"], vec!["c6", "c7", "c8"], vec!["c3", "c4", "c5"]],
    ),
}

fn can_use_exchange_swap_star_impl(jobs_order: Vec<Vec<&str>>, expected: Vec<Vec<&str>>) {
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 9])));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let mut insertion_ctx =
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    rearrange_jobs_in_routes(&mut insertion_ctx, jobs_order.as_slice());
    let vehicles = insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| get_vehicle_id(&route_ctx.route.actor.vehicle).clone())
        .collect::<Vec<_>>();
    assert_eq!(vehicles, vec!["0", "1", "2"]);

    let insertion_ctx = ExchangeSwapStar::new(environment.random.clone(), 1000)
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), expected.as_slice(), "");
}

#[test]
fn can_keep_locked_jobs_in_place() {
    let jobs_order = vec![vec!["c0", "c1", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c8"]];
    let locked_ids = vec!["c2", "c3"];
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 9])));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let mut insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone()),
        locked_ids.as_slice(),
    );
    rearrange_jobs_in_routes(&mut insertion_ctx, jobs_order.as_slice());

    let insertion_ctx = ExchangeSwapStar::new(environment.random.clone(), 1000)
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    let result_ids = get_customer_ids_from_routes(&insertion_ctx);
    assert!(result_ids[0].contains(&"c3".to_string()));
    assert!(!result_ids[0].contains(&"c2".to_string()));

    assert!(result_ids[1].contains(&"c2".to_string()));
    assert!(!result_ids[1].contains(&"c3".to_string()));

    assert!(!result_ids[2].contains(&"c2".to_string()));
    assert!(!result_ids[2].contains(&"c3".to_string()));
}

#[test]
fn can_exchange_jobs_in_routes() {
    let route_pair = (0, 1);
    let disallowed_pairs = vec![];
    let job_order = vec![vec!["c0", "c1", "c3"], vec!["c4", "c5", "c2"]];
    let expected_route_ids = vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"]];

    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());
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
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, false);
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
    let insertion_ctx = create_insertion_ctx(matrix, vec![], false);
    let (leg_selector, result_selector) = create_default_selectors();
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selector, &result_selector);
    let job = get_jobs_by_ids(&insertion_ctx, &[job_id]).first().cloned().unwrap();
    let route_ctx = insertion_ctx.solution.routes.first().unwrap();

    let result = find_insertion_cost(&search_ctx, &job, route_ctx);

    assert_eq!(result, expected);
}

parameterized_test! { can_find_in_place_result, (route_idx, insert_job, extract_job, disallowed_pairs, job_order, expected), {
    can_find_in_place_result_impl(route_idx, insert_job, extract_job, disallowed_pairs, job_order, expected);
}}

can_find_in_place_result! {
    case_01: (0, "c2", "c3", vec![], vec![vec!["c0", "c1", "c3"], vec!["c4", "c5", "c2"]], Some((2., 2))),
    case_02: (0, "c1", "c3", vec![], vec![vec!["c0", "c3", "c2"], vec!["c4", "c5", "c1"]], Some((0., 1))),
    case_03: (0, "c0", "c3", vec![], vec![vec!["c3", "c1", "c2"], vec!["c4", "c5", "c0"]], Some((0., 0))),
    case_04: (0, "c3", "c0", vec![], vec![vec!["c0", "c1", "c2"], vec!["c4", "c5", "c3"]], Some((8., 0))),
}

fn can_find_in_place_result_impl(
    route_idx: usize,
    insert_job: &str,
    extract_job: &str,
    disallowed_pairs: Vec<(&str, &str)>,
    job_order: Vec<Vec<&str>>,
    expected: Option<(Cost, usize)>,
) {
    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());
    let (leg_selector, result_selector) = create_default_selectors();
    let jobs_map = get_jobs_map_by_ids(&insertion_ctx);
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selector, &result_selector);
    let route_ctx = insertion_ctx.solution.routes.get(route_idx).unwrap();
    let insert_job = jobs_map.get(insert_job).unwrap();
    let extract_job = jobs_map.get(extract_job).unwrap();

    let result = find_in_place_result(&search_ctx, route_ctx, insert_job, extract_job)
        .into_success()
        .map(|success| (success.cost, success.activities.first().unwrap().1));

    assert_eq!(result, expected);
}

parameterized_test! { can_find_top_results, (job_id, disallowed_pairs, expected), {
    can_find_top_results_impl(job_id, disallowed_pairs, expected);
}}

can_find_top_results! {
    case_01: ("c5", vec![], vec![Some(5), Some(4), Some(3)]),
    case_02: ("c5", vec![("c3", "c4")], vec![Some(5), Some(3), Some(2)]),
    case_03: ("c5", vec![("cX", "cX")], vec![Some(5), None, None]),
}

fn can_find_top_results_impl(job_id: &str, disallowed_pairs: Vec<(&str, &str)>, expected: Vec<Option<usize>>) {
    let matrix = (5, 2);
    let insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    let (leg_selector, result_selector) = create_default_selectors();
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selector, &result_selector);
    let job_ids = get_jobs_by_ids(&insertion_ctx, &[job_id]);
    let route_ctx = insertion_ctx.solution.routes.first().unwrap();

    let results = find_top_results(&search_ctx, route_ctx, job_ids.as_slice())
        .iter()
        .flat_map(|(_, results)| results.iter())
        .map(|result| result.as_success().map(|success| success.activities.first().unwrap().1))
        .collect::<Vec<_>>();

    assert_eq!(results, expected);
}

parameterized_test! { can_create_route_pairs, (route_pairs_threshold, is_proximity, expected_length), {
    can_create_route_pairs_impl(route_pairs_threshold, is_proximity, expected_length);
}}

can_create_route_pairs! {
    case_01: (9, true, 3),
    case_02: (9, false, 3),
    case_03: (2, true, 2),
    case_04: (2, false, 2),
}

fn can_create_route_pairs_impl(route_pairs_threshold: usize, is_proximity: bool, expected_length: usize) {
    let reals =
        once(if is_proximity { 1 } else { 0 }).chain(vec![0; 9].into_iter()).map(|value| value as f64).collect();
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], reals)));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let pairs = create_route_pairs(&insertion_ctx, route_pairs_threshold);

    assert_eq!(pairs.len(), expected_length);
}
