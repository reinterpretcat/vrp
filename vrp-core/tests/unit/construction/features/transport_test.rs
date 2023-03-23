use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::*;
use crate::models::problem::{VehicleDetail, VehiclePlace};
use std::sync::Arc;

type VehicleData = (Location, Location, Timestamp, Timestamp);
type ActivityData = (Location, (Timestamp, Timestamp), Duration);

const VIOLATION_CODE: ViolationCode = 1;

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
    use super::*;
    use crate::helpers::models::domain::{create_empty_solution_context, create_registry_context};
    use crate::models::solution::{Activity, Place};
    use rosomaxa::prelude::compare_floats;
    use std::cmp::Ordering;

    fn create_feature() -> Feature {
        create_minimize_transport_costs_feature(
            "transport",
            TestTransportCost::new_shared(),
            TestActivityCost::new_shared(),
            VIOLATION_CODE,
        )
        .unwrap()
    }

    fn create_feature_and_route(vehicle_detail_data: VehicleData) -> (Feature, RouteContext) {
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

        (create_feature(), route_ctx)
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

    fn can_properly_calculate_latest_arrival_impl(vehicle_detail_data: VehicleData, activity: usize, time: f64) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data);

        feature.state.unwrap().accept_route_state(&mut route_ctx);

        let activity = route_ctx.route.tour.get(activity).unwrap();
        let result = *route_ctx.state.get_activity_state::<Timestamp>(LATEST_ARRIVAL_KEY, activity).unwrap();

        assert_eq!(result, time);
    }

    parameterized_test! {can_detect_activity_constraint_violation, (vehicle_detail_data, location, prev_index, next_index, expected), {
        can_detect_activity_constraint_violation_impl(vehicle_detail_data, location, prev_index, next_index, expected);
    }}

    can_detect_activity_constraint_violation! {
        case01: ((0, 0, 0., 100.), 50, 3, 4, None),
        case02: ((0, 0, 0., 100.), 1000, 3, 4, ConstraintViolation::skip(VIOLATION_CODE)),
        case03: ((0, 0, 0., 100.), 50, 2, 3, None),
        case04: ((0, 0, 0., 100.), 51, 2, 3, ConstraintViolation::skip(VIOLATION_CODE)),
        case05: ((0, 0, 0., 60.), 40, 3, 4, ConstraintViolation::skip(VIOLATION_CODE)),
        case06: ((0, 0, 0., 50.), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case07: ((0, 0, 0., 10.), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case08: ((0, 0, 60., 100.), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case09: ((0, 40, 0., 40.), 40, 1, 2, ConstraintViolation::skip(VIOLATION_CODE)),
        case10: ((0, 40, 0., 40.), 40, 3, 4, None),
    }

    fn can_detect_activity_constraint_violation_impl(
        vehicle_detail_data: VehicleData,
        location: Location,
        prev_index: usize,
        next_index: usize,
        expected: Option<ConstraintViolation>,
    ) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data);
        feature.state.unwrap().accept_route_state(&mut route_ctx);

        let prev = route_ctx.route.tour.get(prev_index).unwrap();
        let target = test_activity_with_location(location);
        let next = route_ctx.route.tour.get(next_index);
        let activity_ctx = ActivityContext { index: 0, prev, target: &target, next };

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(&route_ctx, &activity_ctx));

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
            registry: create_registry_context(&fleet),
            ..create_empty_solution_context()
        };

        create_feature().state.unwrap().accept_solution_state(&mut solution_ctx);

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

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(&route_ctx, &activity_ctx));

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

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(&route_ctx, &activity_ctx));

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

        let result =
            create_feature().constraint.unwrap().evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

        assert_eq!(result, ConstraintViolation::fail(VIOLATION_CODE));
    }
}

mod time_dependent {
    use super::*;
    use crate::models::problem::{DynamicActivityCost, DynamicTransportCost};
    use hashbrown::HashMap;

    fn create_feature_and_route(
        vehicle_detail_data: VehicleData,
        activities: Vec<ActivityData>,
        reserved_time: TimeWindow,
    ) -> (Feature, RouteContext) {
        let (location_start, location_end, time_start, time_end) = vehicle_detail_data;

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
        let reserved_times = fleet
            .actors
            .first()
            .map(|actor| {
                vec![(actor.clone(), vec![TimeSpan::Window(reserved_time)])].into_iter().collect::<HashMap<_, _>>()
            })
            .unwrap();
        let route_ctx = create_route_context_with_activities(&fleet, "v1", activities);
        let feature = create_minimize_transport_costs_feature(
            "minimize_costs",
            Arc::new(
                DynamicTransportCost::new(reserved_times.clone(), Arc::new(TestTransportCost::default())).unwrap(),
            ),
            Arc::new(DynamicActivityCost::new(reserved_times).unwrap()),
            VIOLATION_CODE,
        )
        .unwrap();

        (feature, route_ctx)
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

    parameterized_test! {can_update_state_for_reserved_time, (vehicle_detail_data, reserved_time, activities, late_arrival_expected, expected_schedules), {
        let reserved_time =  TimeWindow::new(reserved_time.0, reserved_time.1);
        can_update_state_for_reserved_time_impl(vehicle_detail_data, reserved_time, activities, late_arrival_expected, expected_schedules);
    }}

    can_update_state_for_reserved_time! {
        case01_single_outside: ((0, 0, 0., 100.), (25., 30.),
                  vec![(10, (0., 100.), 10.)],
                  vec![None, Some(80.), None],
                  vec![(0., 0.), (10., 20.), (35., 35.)]),

        case02_single_inside: ((0, 0, 0., 100.), (25., 30.),
                  vec![(20, (0., 25.), 10.)],
                  vec![None, Some(20.), None],
                  vec![(0., 0.), (20., 35.), (55., 55.)]),

        case03_two_inside_travel: ((0, 0, 0., 100.), (25., 30.),
                  vec![(10, (0., 20.), 10.), (20, (0., 40.), 10.)],
                  vec![None, Some(15.), Some(40.), None],
                  vec![(0., 0.), (10., 20.), (35., 45.), (65., 65.)]),

        case04_two_inside_service: ((0, 0, 0., 100.), (35., 40.),
                  vec![(10, (0., 20.), 10.), (20, (0., 50.), 10.)],
                  vec![None, Some(15.), Some(50.), None],
                  vec![(0., 0.), (10., 20.), (30., 45.), (65., 65.)]),
    }

    fn can_update_state_for_reserved_time_impl(
        vehicle_detail_data: VehicleData,
        reserved_time: TimeWindow,
        activities: Vec<ActivityData>,
        late_arrival_expected: Vec<Option<f64>>,
        expected_schedules: Vec<(Timestamp, Timestamp)>,
    ) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data, activities, reserved_time);
        feature.state.unwrap().accept_route_state(&mut route_ctx);

        let schedules = get_schedules(&route_ctx);
        let late_arrival_result = get_activity_states(&route_ctx, LATEST_ARRIVAL_KEY);

        assert_eq!(schedules, expected_schedules);
        assert_eq!(late_arrival_result, late_arrival_expected);
    }

    parameterized_test! {can_evaluate_activity, (vehicle_detail_data, reserved_time, target, activities, expected_schedules), {
        let reserved_time =  TimeWindow::new(reserved_time.0, reserved_time.1);
        can_evaluate_activity_impl(vehicle_detail_data, reserved_time, target, activities, expected_schedules);
    }}

    can_evaluate_activity! {
        case01_break_starts_at_target_then_next_at_end:
            ((0, 0, 0., 100.), (10., 20.),
            (10, (0., 100.), 10.),
            vec![(20, (0., 40.), 10.)],
            vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

        case02_break_starts_at_target_then_next_is_late:
            ((0, 0, 0., 100.), (10., 20.),
            (10, (0., 100.), 10.),
            vec![(20, (0., 39.), 10.)],
            vec![]),

        case03_break_with_waiting_time:
            ((0, 0, 0., 100.), (10., 20.),
            (10, (20., 100.), 10.),
            vec![(20, (0., 100.), 10.)],
            vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

        case04_break_during_traveling:
            ((0, 0, 0., 100.), (5., 10.),
            (10, (0., 100.), 10.),
            vec![(20, (0., 100.), 10.)],
            vec![(0., 0.), (15., 25.), (35., 45.), (65., 65.)]),

        case05_break_on_whole_time_window_exclusive:
            ((0, 0, 0., 100.), (10., 20.),
            (10, (0., 20.), 10.),
            vec![(20, (0., 100.), 10.)],
            vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

        case06_break_on_whole_time_window_inclusive:
            ((0, 0, 0., 100.), (10., 21.),
            (10, (0., 20.), 10.),
            vec![(20, (0., 100.), 10.)],
            vec![]),

        case07_break_on_whole_time_window_inclusive:
            ((0, 0, 0., 100.), (9., 21.),
            (10, (10., 20.), 10.),
            vec![(20, (0., 100.), 10.)],
            vec![]),

        case08_break_constraints_next:
            ((0, 0, 0., 100.), (50., 60.),
            (30, (0., 100.), 10.),
            vec![(20, (0., 50.), 10.)],
            vec![]),

        case09_break_constraints_next:
            ((0, 0, 0., 100.), (45., 60.),
            (30, (0., 100.), 10.),
            vec![(20, (0., 50.), 10.)],
            vec![]),
    }

    fn can_evaluate_activity_impl(
        vehicle_detail_data: VehicleData,
        reserved_time: TimeWindow,
        target: ActivityData,
        activities: Vec<ActivityData>,
        expected_schedules: Vec<(Timestamp, Timestamp)>,
    ) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data, activities, reserved_time);
        let feature_constraint = feature.constraint.unwrap();
        let feature_state = feature.state.unwrap();
        feature_state.accept_route_state(&mut route_ctx);
        let (loc, (start, end), dur) = target;
        let prev = route_ctx.route.tour.get(0).unwrap();
        let target = test_activity_with_location_tw_and_duration(loc, TimeWindow::new(start, end), dur);
        let next = route_ctx.route.tour.get(1);
        let activity_ctx = ActivityContext { index: 1, prev, target: &target, next };

        let is_violation = feature_constraint.evaluate(&MoveContext::activity(&route_ctx, &activity_ctx)).is_some();

        assert_eq!(is_violation, expected_schedules.is_empty());
        if !is_violation {
            route_ctx.route_mut().tour.insert_at(target, 1);
            feature_state.accept_route_state(&mut route_ctx);
            assert_eq!(get_schedules(&route_ctx), expected_schedules)
        }
    }
}
