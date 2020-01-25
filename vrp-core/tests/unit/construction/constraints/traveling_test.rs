use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, RouteState};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Distance, Duration, Location};
use std::sync::Arc;

parameterized_test! {can_check_traveling_limits, (vehicle, target, location, limit, expected), {
    can_check_traveling_limits_impl(vehicle, target.to_string(), location, limit, expected);
}}

can_check_traveling_limits! {
    case01: ("v1", "v1", 76, (Some(100.), None), Some(ActivityConstraintViolation { code: 1, stopped: false })),
    case02: ("v1", "v1", 74, (Some(100.), None), None),
    case03: ("v1", "v2", 74, (Some(100.), None), None),

    case04: ("v1", "v1", 76, (None, Some(100.)), Some(ActivityConstraintViolation { code: 2, stopped: false })),
    case05: ("v1", "v1", 74, (None, Some(100.)), None),
    case06: ("v1", "v2", 76, (None, Some(100.)), None),
}

fn can_check_traveling_limits_impl(
    vehicle: &str,
    target: String,
    location: Location,
    limit: (Option<Distance>, Option<Duration>),
    expected: Option<ActivityConstraintViolation>,
) {
    let fleet = FleetBuilder::new().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
    let mut state = RouteState::default();
    state.put_route_state(MAX_DISTANCE_KEY, 50.);
    state.put_route_state(MAX_DURATION_KEY, 50.);
    let route_ctx =
        RouteContext { route: Arc::new(create_route_with_activities(&fleet, vehicle, vec![])), state: Arc::new(state) };
    let pipeline = create_constraint_pipeline_with_module(Box::new(TravelModule::new(
        Arc::new(
            move |actor| {
                if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() {
                    limit
                } else {
                    (None, None)
                }
            },
        ),
        Arc::new(TestTransportCost::new()),
        1,
        2,
    )));

    let result = pipeline.evaluate_hard_activity(
        &route_ctx,
        &ActivityContext {
            index: 0,
            prev: &test_tour_activity_with_location(0),
            target: &test_tour_activity_with_location(location),
            next: Some(&test_tour_activity_with_location(50)),
        },
    );

    assert_eq_option!(result, expected);
}
