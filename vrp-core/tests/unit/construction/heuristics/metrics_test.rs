use crate::construction::features::*;
use crate::construction::heuristics::*;
use crate::helpers::models::domain::create_empty_insertion_context;
use crate::helpers::models::solution::RouteContextBuilder;
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;

fn create_insertion_ctx(route_amount: usize, route_factory: &(dyn Fn(usize) -> RouteContext)) -> InsertionContext {
    let mut ctx = create_empty_insertion_context();
    ctx.solution.routes.extend((0..route_amount).map(route_factory));
    ctx
}

fn create_route_ctx_with_route_state(key: i32, value: f64) -> RouteContext {
    let mut ctx = RouteContextBuilder::default().build();
    ctx.state_mut().put_route_state(key, value);
    ctx
}

#[test]
fn can_get_max_load_variance() {
    let insertion_ctx = create_insertion_ctx(4, &|idx| {
        let value = match idx {
            0 => 5.,
            1 => 3.,
            2 => 0.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(MAX_LOAD_KEY, value)
    });

    let variance = get_max_load_variance(&insertion_ctx);

    assert_eq!(compare_floats(variance, 6.6875), Ordering::Equal);
}

#[test]
fn can_get_duration_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|idx| {
        let value = match idx {
            0 => 6.,
            1 => 2.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(TOTAL_DURATION_KEY, value)
    });

    let mean = get_duration_mean(&insertion_ctx);

    assert_eq!(compare_floats(mean, 5.), Ordering::Equal);
}

#[test]
fn can_get_distance_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|idx| {
        let value = match idx {
            0 => 8.,
            1 => 2.,
            _ => 11.,
        };
        create_route_ctx_with_route_state(TOTAL_DISTANCE_KEY, value)
    });

    let mean = get_distance_mean(&insertion_ctx);

    assert_eq!(compare_floats(mean, 7.), Ordering::Equal);
}
