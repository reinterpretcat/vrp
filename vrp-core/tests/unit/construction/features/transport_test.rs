use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::*;
use crate::models::problem::{VehicleDetail, VehiclePlace};

const VIOLATION_CODE: ViolationCode = ViolationCode(1);
type VehicleData = (Location, Location, Timestamp, Timestamp);

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
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::domain::test_random;
    use crate::models::solution::{Activity, Place, Registry};
    use rosomaxa::prelude::compare_floats;
    use std::cmp::Ordering;

    fn create_feature() -> Feature {
        TransportFeatureBuilder::new("transport")
            .set_violation_code(VIOLATION_CODE)
            .set_transport_cost(TestTransportCost::new_shared())
            .set_activity_cost(TestActivityCost::new_shared())
            .build_minimize_cost()
            .unwrap()
    }

    fn create_feature_and_route(vehicle_detail_data: VehicleData) -> (Feature, RouteContext) {
        let (location_start, location_end, time_start, time_end) = vehicle_detail_data;

        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![TestVehicleBuilder::default()
                .id("v1")
                .details(vec![create_detail((Some(location_start), Some(location_end)), Some((time_start, time_end)))])
                .build()])
            .build();
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(ActivityBuilder::with_location(10).build())
                    .add_activity(ActivityBuilder::with_location(20).build())
                    .add_activity(ActivityBuilder::with_location(30).build())
                    .build(),
            )
            .build();

        let feature = create_feature();

        (feature, route_ctx)
    }

    parameterized_test! {can_properly_calculate_latest_arrival, (vehicle, activity, time), {
        can_properly_calculate_latest_arrival_impl(vehicle, activity, time);
    }}

    can_properly_calculate_latest_arrival! {
        case01: ((0, 0, 0, 100), 3, 70),
        case02: ((0, 0, 0, 100), 2, 60),
        case03: ((0, 0, 0, 100), 1, 50),

        case04: ((0, 0, 0, 60), 3, 30),
        case05: ((0, 0, 0, 60), 2, 20),
        case06: ((0, 0, 0, 60), 1, 10),

        case07: ((40, 40, 0, 100), 3, 90),
        case08: ((40, 40, 0, 100), 1, 70),
        case09: ((40, 40, 0, 100), 2, 80),
    }

    fn can_properly_calculate_latest_arrival_impl(
        vehicle_detail_data: VehicleData,
        activity_idx: usize,
        time: Timestamp,
    ) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data);
        feature.state.unwrap().accept_route_state(&mut route_ctx);

        let result = *route_ctx.state().get_latest_arrival_at(activity_idx).unwrap();

        assert_eq!(result, time);
    }

    parameterized_test! {can_detect_activity_constraint_violation, (vehicle_detail_data, location, prev_index, next_index, expected), {
        can_detect_activity_constraint_violation_impl(vehicle_detail_data, location, prev_index, next_index, expected);
    }}

    can_detect_activity_constraint_violation! {
        case01: ((0, 0, 0, 100), 50, 3, 4, None),
        case02: ((0, 0, 0, 100), 1000, 3, 4, ConstraintViolation::skip(VIOLATION_CODE)),
        case03: ((0, 0, 0, 100), 50, 2, 3, None),
        case04: ((0, 0, 0, 100), 51, 2, 3, ConstraintViolation::skip(VIOLATION_CODE)),
        case05: ((0, 0, 0, 60), 40, 3, 4, ConstraintViolation::skip(VIOLATION_CODE)),
        case06: ((0, 0, 0, 50), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case07: ((0, 0, 0, 10), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case08: ((0, 0, 60, 100), 40, 3, 4, ConstraintViolation::fail(VIOLATION_CODE)),
        case09: ((0, 40, 0, 40), 40, 1, 2, ConstraintViolation::skip(VIOLATION_CODE)),
        case10: ((0, 40, 0, 40), 40, 3, 4, None),
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

        let prev = route_ctx.route().tour.get(prev_index).unwrap();
        let target = ActivityBuilder::with_location(location).build();
        let next = route_ctx.route().tour.get(next_index);
        let activity_ctx = ActivityContext { index: prev_index, prev, target: &target, next };

        let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(&route_ctx, &activity_ctx));

        assert_eq!(result, expected);
    }

    #[test]
    fn can_update_activity_schedule() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes(vec![RouteContextBuilder::default()
                .with_route(
                    RouteBuilder::default()
                        .with_vehicle(&fleet, "v1")
                        .add_activity(
                            ActivityBuilder::default()
                                .place(Place {
                                    idx: 0,
                                    location: 10,
                                    duration: 5,
                                    time: TimeWindow { start: 20, end: 30 },
                                })
                                .schedule(Schedule::new(10, 25))
                                .build(),
                        )
                        .add_activity(
                            ActivityBuilder::default()
                                .place(Place {
                                    idx: 0,
                                    location: 20,
                                    duration: 10,
                                    time: TimeWindow { start: 50, end: 100 },
                                })
                                .schedule(Schedule::new(35, 60))
                                .build(),
                        )
                        .build(),
                )
                .build()])
            .with_registry(Registry::new(&fleet, test_random()))
            .build();
        let mut solution_ctx = insertion_ctx.solution;

        create_feature().state.unwrap().accept_solution_state(&mut solution_ctx);

        let route_ctx = solution_ctx.routes.first().unwrap();
        assert_eq!(route_ctx.route().tour.get(1).unwrap().schedule, Schedule { arrival: 10, departure: 25 });
        assert_eq!(route_ctx.route().tour.get(2).unwrap().schedule, Schedule { arrival: 35, departure: 60 });
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let target = Box::new(Activity {
            place: Place { idx: 0, location: 5, duration: 1, time: DEFAULT_ACTIVITY_TIME_WINDOW },
            schedule: DEFAULT_ACTIVITY_SCHEDULE,
            job: None,
            commute: None,
        });
        let activity_ctx = ActivityContext {
            index: 0,
            prev: route_ctx.route().tour.get(0).unwrap(),
            target: &target,
            next: route_ctx.route().tour.get(1),
        };

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(&route_ctx, &activity_ctx));

        assert_eq!(compare_floats(result, 21.0), Ordering::Equal);
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_non_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(
                        ActivityBuilder::default()
                            .place(Place {
                                idx: 0,
                                location: 10,
                                duration: 0,
                                time: DEFAULT_ACTIVITY_TIME_WINDOW.clone(),
                            })
                            .schedule(Schedule { arrival: 0, departure: 10 })
                            .build(),
                    )
                    .add_activity(
                        ActivityBuilder::default()
                            .place(Place { idx: 0, location: 20, duration: 0, time: TimeWindow { start: 40, end: 70 } })
                            .build(),
                    )
                    .build(),
            )
            .build();
        let target = Box::new(Activity {
            place: Place { idx: 0, location: 30, duration: 10, time: DEFAULT_ACTIVITY_TIME_WINDOW },
            schedule: DEFAULT_ACTIVITY_SCHEDULE,
            job: None,
            commute: None,
        });
        let activity_ctx = ActivityContext {
            index: 0,
            prev: route_ctx.route().tour.get(1).unwrap(),
            target: &target,
            next: route_ctx.route().tour.get(2),
        };

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(&route_ctx, &activity_ctx));

        assert_eq!(compare_floats(result, 30.0), Ordering::Equal);
    }

    #[test]
    fn can_stop_with_time_route_constraint() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let insertion_ctx = TestInsertionContextBuilder::default().build();
        let solution_ctx = insertion_ctx.solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let job = TestSingleBuilder::default().times(vec![TimeWindow::new(2000, 3000)]).build_as_job_ref();

        let result =
            create_feature().constraint.unwrap().evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

        assert_eq!(result, ConstraintViolation::fail(VIOLATION_CODE));
    }
}
