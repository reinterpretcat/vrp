use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
use crate::construction::features::MaxVehicleLoadTourState;
use crate::construction::heuristics::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::solution::RouteContextBuilder;
use crate::models::Problem;

fn create_insertion_ctx(
    route_amount: usize,
    route_factory: &dyn Fn(&Problem, usize) -> RouteContext,
) -> InsertionContext {
    let mut ctx = TestInsertionContextBuilder::default().build();
    let problem = ctx.problem.clone();
    ctx.solution.routes.extend((0..route_amount).map(|idx| route_factory(problem.as_ref(), idx)));
    ctx
}

fn create_route_ctx_with_route_state(state_fn: impl FnOnce(&mut RouteState)) -> RouteContext {
    let mut ctx = RouteContextBuilder::default().build();
    state_fn(ctx.state_mut());
    ctx
}

#[test]
fn can_get_max_load_variance() {
    let insertion_ctx = create_insertion_ctx(4, &|_, idx| {
        let value = match idx {
            0 => 5.,
            1 => 3.,
            2 => 0.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(|state| state.set_max_vehicle_load(value))
    });

    let variance = get_max_load_variance(&insertion_ctx);

    assert_eq!(variance, 6.6875);
}

#[test]
fn can_get_duration_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|_, idx| {
        let value = match idx {
            0 => 6.,
            1 => 2.,
            _ => 7.,
        };
        create_route_ctx_with_route_state(|state| state.set_total_duration(value))
    });

    let mean = get_duration_mean(&insertion_ctx);

    assert_eq!(mean, 5.);
}

#[test]
fn can_get_distance_mean() {
    let insertion_ctx = create_insertion_ctx(3, &|_, idx| {
        let value = match idx {
            0 => 8.,
            1 => 2.,
            _ => 11.,
        };
        create_route_ctx_with_route_state(|state| state.set_total_distance(value))
    });

    let mean = get_distance_mean(&insertion_ctx);

    assert_eq!(mean, 7.);
}
