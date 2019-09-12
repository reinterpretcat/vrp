use crate::construction::constraints::timing::TimingConstraintModule;
use crate::construction::constraints::{ConstraintPipeline, LATEST_ARRIVAL_KEY};
use crate::construction::states::{create_end_activity, create_start_activity, RouteContext, RouteState};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::test_tour_activity_with_location;
use crate::models::common::{Location, Schedule, TimeWindow, Timestamp};
use crate::models::problem::{Fleet, VehicleDetail};
use crate::models::solution::{Activity, Place, Route, Tour};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

fn create_detail(
    locations: (Option<Location>, Option<Location>),
    time: Option<(Timestamp, Timestamp)>,
) -> VehicleDetail {
    VehicleDetail { start: locations.0, end: locations.1, time: time.map(|t| TimeWindow { start: t.0, end: t.1 }) }
}

fn create_route(fleet: &Fleet, vehicle: &str) -> Route {
    let actor = get_test_actor_from_fleet(fleet, vehicle);
    let mut tour = Tour::new();
    tour.set_start(create_start_activity(&actor));
    create_end_activity(&actor).map(|end| tour.set_end(end));

    tour.insert_at(test_tour_activity_with_location(10), 1);
    tour.insert_at(test_tour_activity_with_location(20), 2);
    tour.insert_at(test_tour_activity_with_location(30), 3);

    Route { actor, tour }
}

fn create_constraint_pipeline() -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(TimingConstraintModule::new(
        Arc::new(TestActivityCost::new()),
        Arc::new(TestTransportCost::new()),
        1,
    )));
    constraint
}

parameterized_test! {can_properly_handle_fleet_with_4_vehicles, (vehicle, activity, time), {
    can_properly_handle_fleet_with_4_vehicles_impl(vehicle, activity, time);
}}

can_properly_handle_fleet_with_4_vehicles! {
    case01: ("v1", 3, 70f64),
    case02: ("v2", 3, 30f64),
    case03: ("v3", 3, 90f64),
    case04: ("v4", 3, 90f64),
    case05: ("v1", 2, 60f64),
    case06: ("v2", 2, 20f64),
    case07: ("v3", 2, 80f64),
    case08: ("v4", 2, 80f64),
    case09: ("v1", 1, 50f64),
    case10: ("v2", 1, 10f64),
    case11: ("v3", 1, 70f64),
    case12: ("v4", 1, 70f64),
}

fn can_properly_handle_fleet_with_4_vehicles_impl(vehicle: &str, activity: usize, time: f64) {
    let fleet = Fleet::new(
        vec![test_driver()],
        vec![
            VehicleBuilder::new().id("v1").details(vec![create_detail((Some(0), None), (Some((0.0, 100.0))))]).build(),
            VehicleBuilder::new().id("v2").details(vec![create_detail((Some(0), None), (Some((0.0, 60.0))))]).build(),
            VehicleBuilder::new().id("v3").details(vec![create_detail((Some(40), None), (Some((0.0, 100.0))))]).build(),
            VehicleBuilder::new().id("v4").details(vec![create_detail((Some(40), None), (Some((0.0, 100.0))))]).build(),
        ],
    );
    let mut ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route(&fleet, vehicle))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };

    create_constraint_pipeline().accept_route_state(&mut ctx);
    let result = ctx
        .state
        .read()
        .unwrap()
        .get_activity_state::<Timestamp>(LATEST_ARRIVAL_KEY, ctx.route.read().unwrap().tour.get(activity).unwrap())
        .unwrap()
        .clone();

    assert_eq!(result, time);
}
