use crate::construction::heuristics::*;
use crate::helpers::construction::heuristics::InsertionContextBuilder;
use crate::helpers::models::solution::RouteContextBuilder;
use crate::models::{CoreStateKeys, Problem};
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;

fn create_insertion_ctx(
    route_amount: usize,
    route_factory: &(dyn Fn(&Problem, usize) -> RouteContext),
) -> InsertionContext {
    let mut ctx = InsertionContextBuilder::default().build();
    let problem = ctx.problem.clone();
    ctx.solution.routes.extend((0..route_amount).map(|idx| route_factory(problem.as_ref(), idx)));
    ctx
}

fn create_route_ctx_with_route_state(key: StateKey, value: f64) -> RouteContext {
    let mut ctx = RouteContextBuilder::default().build();
    ctx.state_mut().put_route_state(key, value);
    ctx
}

#[test]
fn can_get_max_load_variance() {
    let insertion_ctx = create_insertion_ctx(4, &|problem, idx| {
        let value = match idx {
            0 => 5.,
            1 => 3.,
            2 => 0.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(problem.extras.get_capacity_keys().unwrap().state_keys.max_load, value)
    });

    let variance = get_max_load_variance(&insertion_ctx);

    assert_eq!(compare_floats(variance, 6.6875), Ordering::Equal);
}

#[test]
fn can_get_duration_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|problem, idx| {
        let value = match idx {
            0 => 6.,
            1 => 2.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(problem.extras.get_schedule_keys().unwrap().total_duration, value)
    });

    let mean = get_duration_mean(&insertion_ctx);

    assert_eq!(compare_floats(mean, 5.), Ordering::Equal);
}

#[test]
fn can_get_distance_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|problem, idx| {
        let value = match idx {
            0 => 8.,
            1 => 2.,
            _ => 11.,
        };
        create_route_ctx_with_route_state(problem.extras.get_schedule_keys().unwrap().total_distance, value)
    });

    let mean = get_distance_mean(&insertion_ctx);

    assert_eq!(compare_floats(mean, 7.), Ordering::Equal);
}
