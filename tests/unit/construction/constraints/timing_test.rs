use crate::construction::constraints::timing::TimingConstraintModule;
use crate::construction::constraints::{ActivityConstraintViolation, ConstraintPipeline, LATEST_ARRIVAL_KEY};
use crate::construction::states::{
    create_end_activity, create_start_activity, ActivityContext, RouteContext, RouteState,
};
use crate::helpers::construction::constraints::create_constraint_pipeline_with_timing;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::{test_tour_activity_with_location, ActivityBuilder, DEFAULT_ACTIVITY_SCHEDULE};
use crate::models::common::{Location, Schedule, TimeWindow, Timestamp};
use crate::models::problem::{Fleet, VehicleDetail};
use crate::models::solution::{Activity, Place, Route, Tour, TourActivity};
use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

fn create_detail(
    locations: (Option<Location>, Option<Location>),
    time: Option<(Timestamp, Timestamp)>,
) -> VehicleDetail {
    VehicleDetail { start: locations.0, end: locations.1, time: time.map(|t| TimeWindow { start: t.0, end: t.1 }) }
}

fn create_route(fleet: &Fleet, vehicle: &str) -> Route {
    create_route_with_activities(
        fleet,
        vehicle,
        vec![
            test_tour_activity_with_location(10),
            test_tour_activity_with_location(20),
            test_tour_activity_with_location(30),
        ],
    )
}

fn create_route_with_activities(fleet: &Fleet, vehicle: &str, activities: Vec<TourActivity>) -> Route {
    let actor = get_test_actor_from_fleet(fleet, vehicle);
    let mut tour = Tour::new();
    tour.set_start(create_start_activity(&actor));
    create_end_activity(&actor).map(|end| tour.set_end(end));

    activities.into_iter().enumerate().for_each(|(index, a)| {
        tour.insert_at(a, index + 1);
    });

    Route { actor, tour }
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

    create_constraint_pipeline_with_timing().accept_route_state(&mut ctx);
    let result = ctx
        .state
        .read()
        .unwrap()
        .get_activity_state::<Timestamp>(LATEST_ARRIVAL_KEY, ctx.route.read().unwrap().tour.get(activity).unwrap())
        .unwrap()
        .clone();

    assert_eq!(result, time);
}

parameterized_test! {can_properly_handle_fleet_with_6_vehicles, (vehicle, location, departure, prev_index, next_index, expected), {
    can_properly_handle_fleet_with_6_vehicles_impl(vehicle, location, departure, prev_index, next_index, expected);
}}

can_properly_handle_fleet_with_6_vehicles! {
    case01: ("v1", 50, 30f64, 3, 4, None),
    case02: ("v1", 1000, 30f64, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
    case03: ("v1", 50, 20f64, 2, 3, None),
    case04: ("v1", 51, 20f64, 2, 3, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
    case05: ("v2", 40, 30f64, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
    case06: ("v3", 40, 30f64, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
    case07: ("v4", 40, 30f64, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
    case08: ("v5", 40, 90f64, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
    case09: ("v6", 40, 30f64, 2, 3, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
    case10: ("v6", 40, 10f64, 1, 2, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
    case11: ("v6", 40, 30f64, 3, 4, None),
}

fn can_properly_handle_fleet_with_6_vehicles_impl(
    vehicle: &str,
    location: Location,
    departure: Timestamp,
    prev_index: usize,
    next_index: usize,
    expected: Option<ActivityConstraintViolation>,
) {
    let fleet = Fleet::new(
        vec![test_driver()],
        vec![
            VehicleBuilder::new()
                .id("v1")
                .details(vec![create_detail((Some(0), Some(0)), (Some((0.0, 100.0))))])
                .build(),
            VehicleBuilder::new()
                .id("v2")
                .details(vec![create_detail((Some(0), Some(0)), (Some((0.0, 60.0))))])
                .build(),
            VehicleBuilder::new()
                .id("v3")
                .details(vec![create_detail((Some(0), Some(0)), (Some((0.0, 50.0))))])
                .build(),
            VehicleBuilder::new()
                .id("v4")
                .details(vec![create_detail((Some(0), Some(0)), (Some((0.0, 10.0))))])
                .build(),
            VehicleBuilder::new()
                .id("v5")
                .details(vec![create_detail((Some(0), Some(0)), (Some((60.0, 100.0))))])
                .build(),
            VehicleBuilder::new()
                .id("v6")
                .details(vec![create_detail((Some(0), Some(40)), (Some((0.0, 40.0))))])
                .build(),
        ],
    );
    let mut route_ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route(&fleet, vehicle))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };
    let pipeline = create_constraint_pipeline_with_timing();
    pipeline.accept_route_state(&mut route_ctx);
    {
        let mut route = route_ctx.route.write().unwrap();
        route
            .tour
            .get_mut(prev_index)
            .map(|a| {
                a.schedule.departure = departure;
                a
            })
            .unwrap();
    };
    let route = route_ctx.route.read().unwrap();
    let prev = route.tour.get(prev_index).unwrap();
    let target = test_tour_activity_with_location(location);
    let next = route.tour.get(next_index);
    let activity_ctx = ActivityContext { index: 0, prev, target: &target, next };

    let result = pipeline.evaluate_hard_activity(&route_ctx, &activity_ctx);

    assert_eq_option!(result, expected);
}

#[test]
fn can_update_activity_schedule() {
    let fleet = Fleet::new(vec![test_driver()], vec![VehicleBuilder::new().id("v1").build()]);
    let mut route_ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route_with_activities(
            &fleet,
            "v1",
            vec![
                Box::new(
                    ActivityBuilder::new()
                        .place(Place { location: 10, duration: 5.0, time: TimeWindow { start: 20.0, end: 30.0 } })
                        .build(),
                ),
                Box::new(
                    ActivityBuilder::new()
                        .place(Place { location: 20, duration: 10.0, time: TimeWindow { start: 50.0, end: 10.0 } })
                        .build(),
                ),
            ],
        ))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };

    create_constraint_pipeline_with_timing().accept_route_state(&mut route_ctx);

    let route = route_ctx.route.read().unwrap();
    assert_eq!(route.tour.get(1).unwrap().schedule, Schedule { arrival: 10.0, departure: 25.0 });
    assert_eq!(route.tour.get(2).unwrap().schedule, Schedule { arrival: 35.0, departure: 60.0 });
}

#[test]
fn can_calculate_soft_activity_cost_for_empty_tour() {
    let fleet = Fleet::new(vec![test_driver_with_costs(empty_costs())], vec![VehicleBuilder::new().id("v1").build()]);
    let mut route_ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route_with_activities(&fleet, "v1", vec![]))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };
    let route = route_ctx.route.read().unwrap();
    let target = Box::new(Activity {
        place: Place { location: 5, duration: 1.0, time: DEFAULT_JOB_TIME_WINDOW },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: None,
    });
    let activity_ctx =
        ActivityContext { index: 0, prev: route.tour.get(0).unwrap(), target: &target, next: route.tour.get(1) };

    let result = create_constraint_pipeline_with_timing().evaluate_soft_activity(&route_ctx, &activity_ctx);

    assert_eq!(compare_floats(&result, &21.0), Ordering::Equal);
}

#[test]
fn can_calculate_soft_activity_cost_for_non_empty_tour() {
    let fleet = Fleet::new(vec![test_driver_with_costs(empty_costs())], vec![VehicleBuilder::new().id("v1").build()]);
    let mut route_ctx = RouteContext {
        route: Arc::new(RwLock::new(create_route_with_activities(
            &fleet,
            "v1",
            vec![
                Box::new(
                    ActivityBuilder::new()
                        .place(Place { location: 10, duration: 0.0, time: DEFAULT_JOB_TIME_WINDOW.clone() })
                        .schedule(Schedule { arrival: 0.0, departure: 10.0 })
                        .build(),
                ),
                Box::new(
                    ActivityBuilder::new()
                        .place(Place { location: 20, duration: 0.0, time: TimeWindow { start: 40.0, end: 70.0 } })
                        .build(),
                ),
            ],
        ))),
        state: Arc::new(RwLock::new(RouteState::new())),
    };
    let route = route_ctx.route.read().unwrap();
    let target = Box::new(Activity {
        place: Place { location: 30, duration: 10.0, time: DEFAULT_JOB_TIME_WINDOW },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: None,
    });
    let activity_ctx =
        ActivityContext { index: 0, prev: route.tour.get(1).unwrap(), target: &target, next: route.tour.get(2) };

    let result = create_constraint_pipeline_with_timing().evaluate_soft_activity(&route_ctx, &activity_ctx);

    assert_eq!(compare_floats(&result, &30.0), Ordering::Equal);
}
