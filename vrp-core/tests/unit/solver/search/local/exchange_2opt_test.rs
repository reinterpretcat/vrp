use super::*;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::solver::*;

parameterized_test! {can_fix_order, (activities, is_open_vrp, job_order, expected), {
    can_fix_order_impl(activities, is_open_vrp, job_order, expected);
}}

can_fix_order! {
    case_01: (3, true, vec![vec!["c1", "c0", "c2"]], vec![vec!["c0", "c1", "c2"]]),
    case_02: (3, false, vec![vec!["c1", "c0", "c2"]], vec![vec!["c0", "c1", "c2"]]),
}

fn can_fix_order_impl(activities: usize, is_open_vrp: bool, job_order: Vec<Vec<&str>>, expected: Vec<Vec<&str>>) {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(activities, 1, is_open_vrp);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());

    let insertion_ctx = ExchangeTwoOpt::default()
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), expected.as_slice(), "");
}
