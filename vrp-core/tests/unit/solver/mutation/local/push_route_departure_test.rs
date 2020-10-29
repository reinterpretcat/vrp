use super::*;
use crate::helpers::models::problem::{SingleBuilder, DEFAULT_ACTIVITY_TIME_WINDOW};
use crate::helpers::solver::{create_default_refinement_ctx, generate_matrix_routes};
use crate::helpers::utils::random::EchoRandom;
use std::sync::Arc;

fn get_test_context(matrix: (usize, usize), is_open_vrp: bool, time_window: TimeWindow) -> InsertionContext {
    let (problem, solution) = generate_matrix_routes(
        matrix.0,
        matrix.1,
        is_open_vrp,
        move |id, location| {
            Arc::new(SingleBuilder::default().id(id).times(vec![time_window.clone()]).location(location).build())
        },
        |v| v,
        |data| (data.clone(), data),
    );

    InsertionContext::new_from_solution(Arc::new(problem), (solution, None), Arc::new(EchoRandom::new(false)))
}

parameterized_test! {can_push_departure_time_in_non_empty_route, (is_open_vrp, offset_ratio, expected_departure), {
    can_push_departure_time_in_non_empty_route_impl(is_open_vrp, offset_ratio, expected_departure);
}}

can_push_departure_time_in_non_empty_route! {
    case_01: (false, 0.5, 3.),
    case_02: (false, 1., 6.),
    case_03: (true, 1., 3.),
}

fn can_push_departure_time_in_non_empty_route_impl(is_open_vrp: bool, offset_ratio: f64, expected_departure: f64) {
    let result_ctx = get_test_context((4, 1), is_open_vrp, TimeWindow::new(0., 1000.));

    let result_ctx = PushRouteDeparture::new(offset_ratio)
        .explore(&create_default_refinement_ctx(result_ctx.problem.clone()), &result_ctx)
        .expect("cannot find solution");

    assert!(result_ctx.solution.unassigned.is_empty());
    let route_ctx = result_ctx.solution.routes.first().unwrap();
    assert_eq!(route_ctx.route.tour.start().unwrap().schedule.departure, expected_departure);
}

parameterized_test! {can_push_departure_time_in_empty_route, (offset_ratio, time_window, expected_departure), {
    let time_window = TimeWindow::new(time_window.0, time_window.1);
    can_push_departure_time_in_empty_route_impl(offset_ratio, time_window, expected_departure);
}}

can_push_departure_time_in_empty_route! {
    case_01: (0.5, (0., 1000.), 500.),
    case_02: (0.1, (100., 500.), 50.),
}

fn can_push_departure_time_in_empty_route_impl(offset_ratio: f64, time_window: TimeWindow, expected_departure: f64) {
    let mut result_ctx = get_test_context((4, 1), true, time_window);
    let route_ctx = result_ctx.solution.routes.first_mut().unwrap();
    let jobs = route_ctx.route.tour.jobs().collect::<Vec<_>>();
    jobs.into_iter().for_each(|job| {
        route_ctx.route_mut().tour.remove(&job);
    });

    let result_ctx = PushRouteDeparture::new(offset_ratio)
        .explore(&create_default_refinement_ctx(result_ctx.problem.clone()), &result_ctx)
        .expect("cannot find solution");

    assert!(result_ctx.solution.unassigned.is_empty());
    let route_ctx = result_ctx.solution.routes.first().unwrap();
    assert_eq!(route_ctx.route.tour.start().unwrap().schedule.departure, expected_departure);
}

#[test]
fn can_handle_empty_solution() {
    let mut result_ctx = get_test_context((4, 1), true, DEFAULT_ACTIVITY_TIME_WINDOW);
    result_ctx.solution.routes.clear();

    let result_ctx =
        PushRouteDeparture::default().explore(&create_default_refinement_ctx(result_ctx.problem.clone()), &result_ctx);

    assert!(result_ctx.is_none());
}

#[test]
fn can_skip_routes_with_locked_jobs() {
    let mut result_ctx = get_test_context((4, 1), true, DEFAULT_ACTIVITY_TIME_WINDOW);
    result_ctx.solution.locked.extend(result_ctx.problem.jobs.all());

    let result_ctx =
        PushRouteDeparture::default().explore(&create_default_refinement_ctx(result_ctx.problem.clone()), &result_ctx);

    assert!(result_ctx.is_none());
}
