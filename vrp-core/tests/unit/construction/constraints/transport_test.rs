use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::*;
use crate::models::problem::{VehicleDetail, VehiclePlace};
use std::sync::Arc;

fn create_detail(
    locations: (Option<Location>, Option<Location>),
    time: Option<(Timestamp, Timestamp)>,
) -> VehicleDetail {
    let (start_location, end_location) = locations;
    VehicleDetail {
        start: start_location.map(|location| VehiclePlace {
            location,
            time: time.map_or(Default::default(), |(start, _)| TimeInterval { earliest: Some(start), latest: None }),
        }),
        end: end_location.map(|location| VehiclePlace {
            location,
            time: time.map_or(Default::default(), |(_, end)| TimeInterval { earliest: None, latest: Some(end) }),
        }),
    }
}

mod timing {
    use super::super::{try_advance_departure_time, try_recede_departure_time};
    use super::*;
    use crate::helpers::construction::constraints::create_constraint_pipeline_with_transport;
    use crate::helpers::models::domain::{create_empty_solution_context, test_random};
    use crate::models::problem::Vehicle;
    use crate::models::solution::{Activity, Place, Registry};
    use rosomaxa::prelude::compare_floats;
    use std::cmp::Ordering;

    fn create_constraint_pipeline_and_route(
        vehicle_detail_data: (Location, Location, Timestamp, Timestamp),
    ) -> (ConstraintPipeline, RouteContext) {
        let (location_start, location_end, time_start, time_end) = vehicle_detail_data;

        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![VehicleBuilder::default()
                .id("v1")
                .details(vec![create_detail((Some(location_start), Some(location_end)), Some((time_start, time_end)))])
                .build()])
            .build();
        let route_ctx = create_route_context_with_activities(
            &fleet,
            "v1",
            vec![test_activity_with_location(10), test_activity_with_location(20), test_activity_with_location(30)],
        );

        (create_constraint_pipeline_with_transport(), route_ctx)
    }

    parameterized_test! {can_properly_calculate_latest_arrival, (vehicle, activity, time), {
        can_properly_calculate_latest_arrival_impl(vehicle, activity, time);
    }}

    can_properly_calculate_latest_arrival! {
        case01: ((0, 0, 0., 100.), 3, 70.),
        case02: ((0, 0, 0., 100.), 2, 60.),
        case03: ((0, 0, 0., 100.), 1, 50.),

        case04: ((0, 0, 0., 60.), 3, 30.),
        case05: ((0, 0, 0., 60.), 2, 20.),
        case06: ((0, 0, 0., 60.), 1, 10.),

        case07: ((40, 40, 0., 100.), 3, 90.),
        case08: ((40, 40, 0., 100.), 1, 70.),
        case09: ((40, 40, 0., 100.), 2, 80.),
    }

    fn can_properly_calculate_latest_arrival_impl(
        vehicle_detail_data: (Location, Location, Timestamp, Timestamp),
        activity: usize,
        time: f64,
    ) {
        let (pipeline, mut route_ctx) = create_constraint_pipeline_and_route(vehicle_detail_data);

        pipeline.accept_route_state(&mut route_ctx);

        let activity = route_ctx.route.tour.get(activity).unwrap();
        let result = *route_ctx.state.get_activity_state::<Timestamp>(LATEST_ARRIVAL_KEY, activity).unwrap();

        assert_eq!(result, time);
    }

    parameterized_test! {can_detect_activity_constraint_violation, (vehicle_detail_data, location, prev_index, next_index, expected), {
        can_detect_activity_constraint_violation_impl(vehicle_detail_data, location, prev_index, next_index, expected);
    }}

    can_detect_activity_constraint_violation! {
        case01: ((0, 0, 0., 100.), 50, 3, 4, None),
        case02: ((0, 0, 0., 100.), 1000, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
        case03: ((0, 0, 0., 100.), 50, 2, 3, None),
        case04: ((0, 0, 0., 100.), 51, 2, 3, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
        case05: ((0, 0, 0., 60.), 40, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
        case06: ((0, 0, 0., 50.), 40, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
        case07: ((0, 0, 0., 10.), 40, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
        case08: ((0, 0, 60., 100.), 40, 3, 4, Some(ActivityConstraintViolation{ code: 1, stopped: true })),
        case09: ((0, 40, 0., 40.), 40, 1, 2, Some(ActivityConstraintViolation{ code: 1, stopped: false })),
        case10: ((0, 40, 0., 40.), 40, 3, 4, None),
    }

    fn can_detect_activity_constraint_violation_impl(
        vehicle_detail_data: (Location, Location, Timestamp, Timestamp),
        location: Location,
        prev_index: usize,
        next_index: usize,
        expected: Option<ActivityConstraintViolation>,
    ) {
        let (pipeline, mut route_ctx) = create_constraint_pipeline_and_route(vehicle_detail_data);
        pipeline.accept_route_state(&mut route_ctx);

        let prev = route_ctx.route.tour.get(prev_index).unwrap();
        let target = test_activity_with_location(location);
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
                    ActivityBuilder::default()
                        .place(Place { location: 10, duration: 5., time: TimeWindow { start: 20., end: 30. } })
                        .schedule(Schedule::new(10., 25.))
                        .build(),
                    ActivityBuilder::default()
                        .place(Place { location: 20, duration: 10., time: TimeWindow { start: 50., end: 100. } })
                        .schedule(Schedule::new(35., 60.))
                        .build(),
                ],
            )],
            registry: RegistryContext::new(Registry::new(&fleet, test_random())),
            ..create_empty_solution_context()
        };

        create_constraint_pipeline_with_transport().accept_solution_state(&mut solution_ctx);

        let route_ctx = solution_ctx.routes.first().unwrap();
        assert_eq!(route_ctx.route.tour.get(1).unwrap().schedule, Schedule { arrival: 10., departure: 25. });
        assert_eq!(route_ctx.route.tour.get(2).unwrap().schedule, Schedule { arrival: 35., departure: 60. });
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
            commute: None,
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
                ActivityBuilder::default()
                    .place(Place { location: 10, duration: 0.0, time: DEFAULT_ACTIVITY_TIME_WINDOW.clone() })
                    .schedule(Schedule { arrival: 0.0, departure: 10.0 })
                    .build(),
                ActivityBuilder::default()
                    .place(Place { location: 20, duration: 0.0, time: TimeWindow { start: 40.0, end: 70.0 } })
                    .build(),
            ],
        );
        let target = Box::new(Activity {
            place: Place { location: 30, duration: 10.0, time: DEFAULT_ACTIVITY_TIME_WINDOW },
            schedule: DEFAULT_ACTIVITY_SCHEDULE,
            job: None,
            commute: None,
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

    parameterized_test! {can_advance_departure_time, (latest, optimize_whole_tour, tws, expected), {
        let tws = tws.into_iter().map(|(start, end)| TimeWindow::new(start, end)).collect::<Vec<_>>();
        can_advance_departure_time_impl(latest, optimize_whole_tour, tws, expected);
    }}

    can_advance_departure_time! {
        case01: (None, true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(5.)),
        case02: (Some(3.), true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(3.)),
        case03: (Some(7.), true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(5.)),
        case04: (None, true, vec![(0., 100.), (10., 100.), (42., 100.)], Some(12.)),

        case05: (None, false, vec![(12., 100.), (0., 100.), (0., 100.)], Some(2.)),
        case06: (None, false, vec![(10., 100.), (0., 100.), (0., 100.)], None),
        case07: (None, false, vec![(0., 100.), (25., 100.), (0., 100.)], None),
    }

    fn can_advance_departure_time_impl(
        latest: Option<f64>,
        optimize_whole_tour: bool,
        tws: Vec<TimeWindow>,
        expected: Option<f64>,
    ) {
        if let [tw1, tw2, tw3] = tws.as_slice() {
            let fleet = FleetBuilder::default()
                .add_driver(test_driver())
                .add_vehicle(Vehicle {
                    details: vec![VehicleDetail {
                        start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.), latest } }),
                        ..test_vehicle_detail()
                    }],
                    ..test_vehicle_with_id("v1")
                })
                .build();
            let route_ctx = create_route_context_with_activities(
                &fleet,
                "v1",
                vec![
                    test_activity_with_location_and_tw(10, tw1.clone()),
                    test_activity_with_location_and_tw(20, tw2.clone()),
                    test_activity_with_location_and_tw(30, tw3.clone()),
                ],
            );

            let departure_time =
                try_advance_departure_time(&route_ctx, &TestTransportCost::default(), optimize_whole_tour);

            assert_eq!(departure_time, expected);
        } else {
            unreachable!()
        }
    }

    parameterized_test! {can_recede_departure_time, (earliest, start_departure, latest_first_arrival, tw, duration_limit, expected), {
        can_recede_departure_time_impl(earliest, start_departure, latest_first_arrival, TimeWindow::new(tw.0, tw.1), duration_limit, expected);
    }}

    can_recede_departure_time! {
        case01: (Some(0.), 0., 10., (10., 20.), None, None),
        case02: (Some(0.), 5., 10., (10., 20.), None, None),
        case03: (Some(5.), 10., 15., (10., 20.), None, Some(5.)),
        case04: (Some(5.), 10., 20., (10., 20.), None, Some(5.)),
        case05: (None, 10., 50., (10., 20.), None, Some(0.)),
        case06: (Some(5.), 10., 11., (10., 20.), None, Some(9.)),

        case07: (Some(0.), 10., 20., (10., 20.), Some((20., 30.)), Some(0.)),
        case08: (Some(0.), 10., 20., (10., 20.), Some((20., 25.)), Some(5.)),
        case09: (Some(0.), 10., 20., (10., 20.), Some((20., 20.)), None),
    }

    fn can_recede_departure_time_impl(
        earliest: Option<f64>,
        start_departure: f64,
        latest_first_arrival: f64,
        tw: TimeWindow,
        total_duration_limit: Option<(f64, f64)>,
        expected: Option<f64>,
    ) {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicle(Vehicle {
                details: vec![VehicleDetail {
                    start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest, latest: None } }),
                    ..test_vehicle_detail()
                }],
                ..test_vehicle_with_id("v1")
            })
            .build();
        let mut route_ctx =
            create_route_context_with_activities(&fleet, "v1", vec![test_activity_with_location_and_tw(10, tw)]);
        let (route, state) = route_ctx.as_mut();
        route.tour.get_mut(0).unwrap().schedule.departure = start_departure;
        let first = route.tour.get(1).unwrap();
        state.put_activity_state::<f64>(LATEST_ARRIVAL_KEY, first, latest_first_arrival);

        if let Some((total, limit)) = total_duration_limit {
            state.put_route_state::<f64>(TOTAL_DURATION_KEY, total);
            state.put_route_state::<f64>(LIMIT_DURATION_KEY, limit);
        }

        let departure_time = try_recede_departure_time(&route_ctx);

        assert_eq!(departure_time, expected);
    }
}

mod traveling {
    use super::super::stop;
    use super::*;
    use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;

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
        let route_ctx = RouteContext::new_with_state(
            Arc::new(create_route_with_activities(&fleet, vehicle, vec![])),
            Arc::new(state),
        );
        let pipeline = create_constraint_pipeline_with_module(Arc::new(TransportConstraintModule::new(
            TestTransportCost::new_shared(),
            Arc::new(TestActivityCost::default()),
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
                prev: &test_activity_with_location(50),
                target: &test_activity_with_location(location),
                next: Some(&test_activity_with_location(50)),
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
                prev: &test_activity_with_location(50),
                target: &test_activity_with_location_and_tw(75, TimeWindow::new(100., 100.)),
                next: Some(&test_activity_with_location(50)),
            },
        );

        assert_eq!(result, stop(3));
    }
}

mod time_dependent {
    use super::*;
    use crate::helpers::construction::constraints::create_constraint_pipeline_with_module;
    use crate::models::problem::{Actor, TravelTime};
    use crate::models::solution::Activity;
    use std::ops::Deref;

    struct DynamicActivityCost {
        reserved_time_func: Arc<dyn Fn(&Actor, &TimeWindow) -> Option<TimeWindow> + Send + Sync>,
    }

    impl ActivityCost for DynamicActivityCost {
        fn estimate_departure(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp {
            let activity_start = arrival.max(activity.place.time.start);
            let departure = activity_start + activity.place.duration;
            let schedule = TimeWindow::new(arrival, departure);

            // use cases to validate:
            //
            // 1. condition:   break starts after arrival, but before time window start and it ends after time window end
            //    expectation: disallow insertion
            //    NOTE: should be handled by estimate_arrival api to return earliest arrival time as be time?
            // arr----------tws----------twe------
            //    bs---------------------------be
            //
            // arr----------tws----------twe------
            //    bs----be
            //
            // arr----------tws----------twe------
            //    bs------------be
            //
            // arr----------tws----------twe------
            //                  bs------------be
            //
            // arr----------tws----------twe------
            //    bs---------------------------be

            self.reserved_time_func.deref()(actor, &schedule).map_or(departure, |reserved_time: TimeWindow| {
                assert!(reserved_time.intersects(&schedule));

                let time_window = &activity.place.time;

                let extra_duration = if reserved_time.start < time_window.start {
                    let waiting_time = TimeWindow::new(arrival, time_window.start);
                    let overlapping = waiting_time.overlapping(&reserved_time).map(|tw| tw.duration()).unwrap_or(0.);

                    reserved_time.duration() - overlapping
                } else {
                    reserved_time.duration()
                };

                departure + extra_duration
            })
        }

        fn estimate_arrival(&self, actor: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp {
            let arrival = departure - activity.place.duration;
            let schedule = TimeWindow::new(arrival, departure);

            self.reserved_time_func.deref()(actor, &schedule).map_or(arrival, |reserved_time: TimeWindow| {
                // TODO consider overlapping break with waiting time?
                arrival - reserved_time.duration()
            })
        }
    }

    struct DynamicTransportCost {
        reserved_time_func: Arc<dyn Fn(&Actor, &TimeWindow) -> Option<TimeWindow> + Send + Sync>,
        inner: TestTransportCost,
    }

    impl TransportCost for DynamicTransportCost {
        fn duration_approx(&self, profile: &Profile, from: Location, to: Location) -> Duration {
            self.inner.duration_approx(profile, from, to)
        }

        fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
            self.inner.distance_approx(profile, from, to)
        }

        fn duration(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Duration {
            let duration = self.inner.duration(actor, from, to, travel_time);

            let time_window = match travel_time {
                TravelTime::Arrival(arrival) => TimeWindow::new(arrival - duration, arrival),
                TravelTime::Departure(departure) => TimeWindow::new(departure, departure + duration),
            };

            self.reserved_time_func.deref()(actor, &time_window)
                .map_or(duration, |reserved_time: TimeWindow| duration + reserved_time.duration())
        }

        fn distance(&self, actor: &Actor, from: Location, to: Location, travel_time: TravelTime) -> Distance {
            self.inner.distance(actor, from, to, travel_time)
        }
    }

    fn create_constraint_pipeline_and_route(
        vehicle_detail_data: (Location, Location, Timestamp, Timestamp),
        activities: Vec<(Location, (Timestamp, Timestamp), Duration)>,
        reserved_time: TimeWindow,
    ) -> (ConstraintPipeline, RouteContext) {
        let (location_start, location_end, time_start, time_end) = vehicle_detail_data;

        let reserved_time_func =
            Arc::new(
                move |_: &Actor, time_window: &TimeWindow| {
                    if time_window.intersects(&reserved_time) {
                        Some(reserved_time.clone())
                    } else {
                        None
                    }
                },
            );

        let activities = activities
            .into_iter()
            .map(|(loc, (start, end), dur)| {
                test_activity_with_location_tw_and_duration(loc, TimeWindow::new(start, end), dur)
            })
            .collect();

        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![VehicleBuilder::default()
                .id("v1")
                .details(vec![create_detail((Some(location_start), Some(location_end)), Some((time_start, time_end)))])
                .build()])
            .build();
        let route_ctx = create_route_context_with_activities(&fleet, "v1", activities);

        let pipeline = create_constraint_pipeline_with_module(Arc::new(TransportConstraintModule::new(
            Arc::new(DynamicTransportCost {
                reserved_time_func: reserved_time_func.clone(),
                inner: TestTransportCost::default(),
            }),
            Arc::new(DynamicActivityCost { reserved_time_func }),
            Arc::new(|_| (None, None)),
            1,
            2,
            3,
        )));

        (pipeline, route_ctx)
    }

    fn get_activity_states(route_ctx: &RouteContext, key: i32) -> Vec<Option<f64>> {
        route_ctx
            .route
            .tour
            .all_activities()
            .map(|a| route_ctx.state.get_activity_state::<f64>(key, a).cloned())
            .collect()
    }

    fn get_schedules(route_ctx: &RouteContext) -> Vec<(Timestamp, Timestamp)> {
        route_ctx.route.tour.all_activities().map(|a| (a.schedule.arrival, a.schedule.departure)).collect()
    }

    parameterized_test! {can_update_state_for_reserved_time, (vehicle_detail_data, reserved_time, activities, late_arrival_expected, schedules_expected), {
        let reserved_time =  TimeWindow::new(reserved_time.0, reserved_time.1);
        can_update_state_for_reserved_time_impl(vehicle_detail_data, reserved_time, activities, late_arrival_expected, schedules_expected);
    }}

    can_update_state_for_reserved_time! {
        case01_single_outside: ((0, 0, 0., 100.), (25., 30.),
                  vec![(10, (0., 100.), 10.)],
                  vec![None, Some(80.), None],
                  vec![(0., 0.), (10., 20.), (35., 35.)]),

        case02_single_inside: ((0, 0, 0., 100.), (25., 30.),
                  vec![(20, (0., 25.), 10.)],
                  vec![None, Some(25.), None],
                  vec![(0., 0.), (20., 35.), (55., 55.)]),

        case03_two_inside_travel: ((0, 0, 0., 100.), (25., 30.),
                  vec![(10, (0., 20.), 10.), (20, (0., 40.), 10.)],
                  vec![None, Some(10.), Some(40.), None],
                  vec![(0., 0.), (10., 20.), (35., 45.), (65., 65.)]),

        case04_two_inside_service: ((0, 0, 0., 100.), (35., 40.),
                  vec![(10, (0., 20.), 10.), (20, (0., 50.), 10.)],
                  vec![None, Some(20.), Some(50.), None],
                  vec![(0., 0.), (10., 20.), (30., 45.), (65., 65.)]),
    }

    fn can_update_state_for_reserved_time_impl(
        vehicle_detail_data: (Location, Location, Timestamp, Timestamp),
        reserved_time: TimeWindow,
        activities: Vec<(Location, (Timestamp, Timestamp), Duration)>,
        late_arrival_expected: Vec<Option<f64>>,
        schedules_expected: Vec<(Timestamp, Timestamp)>,
    ) {
        let (pipeline, mut route_ctx) =
            create_constraint_pipeline_and_route(vehicle_detail_data, activities, reserved_time);
        pipeline.accept_route_state(&mut route_ctx);

        let schedules = get_schedules(&route_ctx);
        let late_arrival_result = get_activity_states(&route_ctx, LATEST_ARRIVAL_KEY);

        assert_eq!(schedules, schedules_expected);
        assert_eq!(late_arrival_result, late_arrival_expected);
    }
}
