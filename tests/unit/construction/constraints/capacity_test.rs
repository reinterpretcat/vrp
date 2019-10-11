use crate::construction::constraints::capacity::CURRENT_CAPACITY_KEY;
use crate::construction::constraints::Demand;
use crate::construction::states::{RouteContext, RouteState};
use crate::helpers::construction::constraints::*;
use crate::helpers::models::problem::{test_driver, VehicleBuilder};
use crate::helpers::models::solution::*;
use crate::models::common::TimeWindow;
use crate::models::problem::{Fleet, Vehicle, VehicleDetail};
use crate::models::solution::{Activity, TourActivity};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

fn create_test_vehicle(capacity: i32) -> Vehicle {
    VehicleBuilder::new().id("v1").capacity(capacity).build()
}

fn create_demand(size: i32) -> Demand<i32> {
    if size > 0 {
        Demand::<i32> { pickup: (size, 0), delivery: (0, 0) }
    } else {
        Demand::<i32> { pickup: (0, 0), delivery: (-size, 0) }
    }
}

fn get_simple_capacity_state(key: i32, state: &RouteState, activity: Option<&TourActivity>) -> i32 {
    *state.get_activity_state::<i32>(key, activity.unwrap()).unwrap()
}

parameterized_test! {can_calculate_current_capacity_state_values, (s1, s2, s3, start, end, exp_s1, exp_s2, exp_s3), {
    can_calculate_current_capacity_state_values_impl(s1, s2, s3, start, end, exp_s1, exp_s2, exp_s3);
}}

can_calculate_current_capacity_state_values! {
    case01: (-1, 2, -3, 4, 2, 3, 5, 2),
    case02: (1, -2, 3, 2, 4, 3, 1, 4),
    case03: (0, 1, 0, 0, 1, 0, 1, 1),
}

fn can_calculate_current_capacity_state_values_impl(
    s1: i32,
    s2: i32,
    s3: i32,
    start: i32,
    end: i32,
    exp_s1: i32,
    exp_s2: i32,
    exp_s3: i32,
) {
    let fleet = Fleet::new(vec![test_driver()], vec![create_test_vehicle(10)]);
    let mut ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route_with_activities(
            &fleet,
            "v1",
            vec![
                test_tour_activity_with_simple_demand(create_demand(s1)),
                test_tour_activity_with_simple_demand(create_demand(s2)),
                test_tour_activity_with_simple_demand(create_demand(s3)),
            ],
        ))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };

    create_constraint_pipeline_with_simple_capacity().accept_route_state(&mut ctx);

    let tour = &ctx.route.read().unwrap().tour;
    let state = ctx.state.read().unwrap();
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, &state, tour.start()), start);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, &state, tour.end()), end);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, &state, tour.get(1)), exp_s1);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, &state, tour.get(2)), exp_s2);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, &state, tour.get(3)), exp_s3);
}
