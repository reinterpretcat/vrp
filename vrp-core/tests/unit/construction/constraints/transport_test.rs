mod timing {
    use crate::construction::constraints::{ActivityConstraintViolation, RouteConstraintViolation};
    use crate::construction::heuristics::*;
    use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
    use crate::helpers::models::domain::create_empty_solution_context;
    use crate::helpers::models::problem::*;
    use crate::helpers::models::solution::*;
    use crate::models::common::{Location, Schedule, TimeWindow, Timestamp};
    use crate::models::problem::{Fleet, VehicleDetail};
    use crate::models::solution::{Activity, Place, Registry};
    use crate::utils::compare_floats;
    use std::cmp::Ordering;

    fn create_detail(
        locations: (Option<Location>, Option<Location>),
        time: Option<(Timestamp, Timestamp)>,
    ) -> VehicleDetail {
        VehicleDetail { start: locations.0, end: locations.1, time: time.map(|t| TimeWindow { start: t.0, end: t.1 }) }
    }

    fn create_route_context(fleet: &Fleet, vehicle: &str) -> RouteContext {
        create_route_context_with_activities(
            fleet,
            vehicle,
            vec![
                test_tour_activity_with_location(10),
                test_tour_activity_with_location(20),
                test_tour_activity_with_location(30),
            ],
        )
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
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![
                VehicleBuilder::default()
                    .id("v1")
                    .details(vec![create_detail((Some(0), None), Some((0.0, 100.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v2")
                    .details(vec![create_detail((Some(0), None), Some((0.0, 60.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v3")
                    .details(vec![create_detail((Some(40), None), Some((0.0, 100.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v4")
                    .details(vec![create_detail((Some(40), None), Some((0.0, 100.0)))])
                    .build(),
            ])
            .build();
        let mut ctx = create_route_context(&fleet, vehicle);

        create_constraint_pipeline_with_transport().accept_route_state(&mut ctx);
        let result = *ctx.state.get_activity_state::<Timestamp>(1, ctx.route.tour.get(activity).unwrap()).unwrap();

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
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![
                VehicleBuilder::default()
                    .id("v1")
                    .details(vec![create_detail((Some(0), Some(0)), Some((0.0, 100.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v2")
                    .details(vec![create_detail((Some(0), Some(0)), Some((0.0, 60.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v3")
                    .details(vec![create_detail((Some(0), Some(0)), Some((0.0, 50.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v4")
                    .details(vec![create_detail((Some(0), Some(0)), Some((0.0, 10.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v5")
                    .details(vec![create_detail((Some(0), Some(0)), Some((60.0, 100.0)))])
                    .build(),
                VehicleBuilder::default()
                    .id("v6")
                    .details(vec![create_detail((Some(0), Some(40)), Some((0.0, 40.0)))])
                    .build(),
            ])
            .build();
        let mut route_ctx = create_route_context(&fleet, vehicle);
        let pipeline = create_constraint_pipeline_with_transport();
        pipeline.accept_route_state(&mut route_ctx);
        route_ctx
            .route_mut()
            .tour
            .get_mut(prev_index)
            .map(|a| {
                a.schedule.departure = departure;
                a
            })
            .unwrap();

        let prev = route_ctx.route.tour.get(prev_index).unwrap();
        let target = test_tour_activity_with_location(location);
        let next = route_ctx.route.tour.get(next_index);
        let activity_ctx = ActivityContext { index: 0, prev, target: &target, next };

        let result = pipeline.evaluate_hard_activity(&route_ctx, &activity_ctx);

        assert_eq!(result, expected);
    }

    #[test]
    fn can_update_activity_schedule() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![VehicleBuilder::default().id("v1").build()])
            .build();
        let mut solution_ctx = SolutionContext {
            routes: vec![create_route_context_with_activities(
                &fleet,
                "v1",
                vec![
                    Box::new(
                        ActivityBuilder::default()
                            .place(Place { location: 10, duration: 5.0, time: TimeWindow { start: 20.0, end: 30.0 } })
                            .build(),
                    ),
                    Box::new(
                        ActivityBuilder::default()
                            .place(Place { location: 20, duration: 10.0, time: TimeWindow { start: 50.0, end: 10.0 } })
                            .build(),
                    ),
                ],
            )],
            registry: Registry::new(&fleet),
            ..create_empty_solution_context()
        };

        create_constraint_pipeline_with_transport().accept_solution_state(&mut solution_ctx);

        let route_ctx = solution_ctx.routes.first().unwrap();
        assert_eq!(route_ctx.route.tour.get(1).unwrap().schedule, Schedule { arrival: 20.0, departure: 25.0 });
        assert_eq!(route_ctx.route.tour.get(2).unwrap().schedule, Schedule { arrival: 35.0, departure: 60.0 });
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![VehicleBuilder::default().id("v1").build()])
            .build();
        let route_ctx = create_route_context_with_activities(&fleet, "v1", vec![]);
        let target = Box::new(Activity {
            place: Place { location: 5, duration: 1.0, time: DEFAULT_ACTIVITY_TIME_WINDOW },
            schedule: DEFAULT_ACTIVITY_SCHEDULE,
            job: None,
        });
        let activity_ctx = ActivityContext {
            index: 0,
            prev: route_ctx.route.tour.get(0).unwrap(),
            target: &target,
            next: route_ctx.route.tour.get(1),
        };

        let result = create_constraint_pipeline_with_transport().evaluate_soft_activity(&route_ctx, &activity_ctx);

        assert_eq!(compare_floats(result, 21.0), Ordering::Equal);
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_non_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![VehicleBuilder::default().id("v1").build()])
            .build();
        let route_ctx = create_route_context_with_activities(
            &fleet,
            "v1",
            vec![
                Box::new(
                    ActivityBuilder::default()
                        .place(Place { location: 10, duration: 0.0, time: DEFAULT_ACTIVITY_TIME_WINDOW.clone() })
                        .schedule(Schedule { arrival: 0.0, departure: 10.0 })
                        .build(),
                ),
                Box::new(
                    ActivityBuilder::default()
                        .place(Place { location: 20, duration: 0.0, time: TimeWindow { start: 40.0, end: 70.0 } })
                        .build(),
                ),
            ],
        );
        let target = Box::new(Activity {
            place: Place { location: 30, duration: 10.0, time: DEFAULT_ACTIVITY_TIME_WINDOW },
            schedule: DEFAULT_ACTIVITY_SCHEDULE,
            job: None,
        });
        let activity_ctx = ActivityContext {
            index: 0,
            prev: route_ctx.route.tour.get(1).unwrap(),
            target: &target,
            next: route_ctx.route.tour.get(2),
        };

        let result = create_constraint_pipeline_with_transport().evaluate_soft_activity(&route_ctx, &activity_ctx);

        assert_eq!(compare_floats(result, 30.0), Ordering::Equal);
    }

    #[test]
    fn can_stop_with_time_route_constraint() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![VehicleBuilder::default().id("v1").build()])
            .build();
        let solution_ctx = create_empty_solution_context();
        let route_ctx = create_route_context_with_activities(&fleet, "v1", vec![]);
        let job = SingleBuilder::default().times(vec![TimeWindow::new(2000., 3000.)]).build_as_job_ref();

        let result = create_constraint_pipeline_with_transport().evaluate_hard_route(&solution_ctx, &route_ctx, &job);

        assert_eq!(result, Some(RouteConstraintViolation { code: 1 }));
    }
}

mod traveling {
    use super::super::stop;
    use crate::construction::constraints::*;
    use crate::construction::heuristics::{ActivityContext, RouteContext, RouteState};
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
        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();
        let mut state = RouteState::default();
        state.put_route_state(TOTAL_DISTANCE_KEY, 50.);
        state.put_route_state(TOTAL_DURATION_KEY, 50.);
        let target = target.to_owned();
        let route_ctx = RouteContext {
            route: Arc::new(create_route_with_activities(&fleet, vehicle, vec![])),
            state: Arc::new(state),
        };
        let pipeline = create_constraint_pipeline_with_module(Box::new(TransportConstraintModule::new(
            Arc::new(TestActivityCost::default()),
            TestTransportCost::new_shared(),
            Arc::new(
                move |actor| {
                    if get_vehicle_id(actor.vehicle.as_ref()) == target.as_str() {
                        limit
                    } else {
                        (None, None)
                    }
                },
            ),
            1,
            2,
            3,
        )));

        (pipeline, route_ctx)
    }

    parameterized_test! {can_check_traveling_limits, (vehicle, target, location, limit, expected), {
        can_check_traveling_limits_impl(vehicle, target, location, limit, expected);
    }}

    can_check_traveling_limits! {
        case01: ("v1", "v1", 76, (Some(100.), None), stop(2)),
        case02: ("v1", "v1", 74, (Some(100.), None), None),
        case03: ("v1", "v2", 76, (Some(100.), None), None),

        case04: ("v1", "v1", 76, (None, Some(100.)), stop(3)),
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

        assert_eq!(result, expected);
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
                next: Some(&test_tour_activity_with_location(50)),
            },
        );

        assert_eq!(result, stop(3));
    }
}
