use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::*;
use std::cmp::Ordering;

fn create_test_insertion_ctx(routes: &[f64]) -> InsertionContext {
    let mut insertion_ctx = TestInsertionContextBuilder::default().build();
    let problem = insertion_ctx.problem.clone();

    routes.iter().for_each(|arrival| {
        let mut route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(problem.fleet.as_ref(), "v1").build())
            .build();
        route_ctx.route_mut().tour.all_activities_mut().last().unwrap().schedule.arrival = *arrival;

        insertion_ctx.solution.routes.push(route_ctx);
    });

    insertion_ctx
}

parameterized_test! {can_properly_estimate_solutions, (left, right, expected), {
    can_properly_estimate_solutions_impl(left, right, expected);
}}

can_properly_estimate_solutions! {
    case_01: (&[10.], &[10.], Ordering::Equal),
    case_02: (&[10.], &[11.], Ordering::Less),
    case_03: (&[10.], &[9.], Ordering::Greater),
    case_04: (&[10.], &[10., 10.], Ordering::Equal),
    case_05: (&[10.], &[10., 9.], Ordering::Greater),
    case_06: (&[10.], &[10., 11.], Ordering::Less),
}

fn can_properly_estimate_solutions_impl(left: &[f64], right: &[f64], expected: Ordering) {
    let left = create_test_insertion_ctx(left);
    let right = create_test_insertion_ctx(right);
    let objective = create_minimize_arrival_time_feature("minimize_arrival").unwrap().objective.unwrap();

    let left = objective.fitness(&left);
    let right = objective.fitness(&right);

    assert_eq!(left.total_cmp(&right), expected);
}
