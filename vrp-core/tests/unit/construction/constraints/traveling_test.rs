use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, RouteState};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Distance, Duration, Location, TimeWindow};
use std::sync::Arc;

fn create_test_data(
    vehicle: &str,
    target: &str,
    limit: (Option<Distance>, Option<Duration>),
) -> (ConstraintPipeline, RouteContext) {
    let fleet = FleetBuilder::new().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
    let mut state = RouteState::default();
    state.put_route_state(MAX_DISTANCE_KEY, 50.);
    state.put_route_state(MAX_DURATION_KEY, 50.);
    let target = target.to_owned();
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

    (pipeline, route_ctx)
}

parameterized_test! {can_check_traveling_limits, (vehicle, target, location, limit, expected), {
    can_check_traveling_limits_impl(vehicle, target, location, limit, expected);
}}

can_check_traveling_limits! {
    case01: ("v1", "v1", 76, (Some(100.), None), Some(ActivityConstraintViolation { code: 1, stopped: false })),
    case02: ("v1", "v1", 74, (Some(100.), None), None),
    case03: ("v1", "v2", 76, (Some(100.), None), None),

    case04: ("v1", "v1", 76, (None, Some(100.)), Some(ActivityConstraintViolation { code: 2, stopped: false })),
    case05: ("v1", "v1", 74, (None, Some(100.)), None),
    case06: ("v1", "v2", 76, (None, Some(100.)), None),
}

fn can_check_traveling_limits_impl(
    vehicle: &str,
    target: &str,
    location: Location,
    limit: (Option<Distance>, Option<Duration>),
    expected: Option<ActivityConstraintViolation>,
) {
    let (pipeline, route_ctx) = create_test_data(vehicle, target, limit);

    let result = pipeline.evaluate_hard_activity(
        &route_ctx,
        &ActivityContext {
            index: 0,
            prev: &test_tour_activity_with_location(50),
            target: &test_tour_activity_with_location(location),
            next: Some(&test_tour_activity_with_location(50)),
        },
    );

    assert_eq_option!(result, expected);
}

#[test]
fn can_consider_waiting_time() {
    let (pipeline, route_ctx) = create_test_data("v1", "v1", (None, Some(100.)));

    let result = pipeline.evaluate_hard_activity(
        &route_ctx,
        &ActivityContext {
            index: 0,
            prev: &test_tour_activity_with_location(50),
            target: &test_tour_activity_with_location_and_tw(75, TimeWindow::new(100., 100.)),
            next: Some(&test_tour_activity_with_location(100)),
        },
    );

    assert_eq_option!(result, Some(ActivityConstraintViolation { code: 2, stopped: false }));
}
