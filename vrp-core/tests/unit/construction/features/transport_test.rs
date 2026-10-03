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
            .add_vehicles(vec![
                TestVehicleBuilder::default()
                    .id("v1")
                    .details(vec![create_detail(
                        (Some(location_start), Some(location_end)),
                        Some((time_start, time_end)),
                    )])
                    .build(),
            ])
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

    fn can_properly_calculate_latest_arrival_impl(vehicle_detail_data: VehicleData, activity_idx: usize, time: Float) {
        let (feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data);
        feature.state.unwrap().accept_route_state(&mut route_ctx);

        let result = *route_ctx.state().get_latest_arrival_at(activity_idx).unwrap();

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
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;

        let prev = route_ctx.route().tour.get(prev_index).unwrap();
        let target = ActivityBuilder::with_location(location).build();
        let next = route_ctx.route().tour.get(next_index);
        let activity_ctx = ActivityContext { index: prev_index, prev, target: &target, next };

        let result =
            feature.constraint.unwrap().evaluate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx));

        assert_eq!(result, expected);
    }

    #[test]
    fn can_update_activity_schedule() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes(vec![
                RouteContextBuilder::default()
                    .with_route(
                        RouteBuilder::default()
                            .with_vehicle(&fleet, "v1")
                            .add_activity(
                                ActivityBuilder::default()
                                    .place(Place {
                                        idx: 0,
                                        location: 10,
                                        duration: 5.,
                                        time: TimeWindow { start: 20., end: 30. },
                                    })
                                    .schedule(Schedule::new(10., 25.))
                                    .build(),
                            )
                            .add_activity(
                                ActivityBuilder::default()
                                    .place(Place {
                                        idx: 0,
                                        location: 20,
                                        duration: 10.,
                                        time: TimeWindow { start: 50., end: 100. },
                                    })
                                    .schedule(Schedule::new(35., 60.))
                                    .build(),
                            )
                            .build(),
                    )
                    .build(),
            ])
            .with_registry(Registry::new(&fleet, test_random()))
            .build();
        let mut solution_ctx = insertion_ctx.solution;

        create_feature().state.unwrap().accept_solution_state(&mut solution_ctx);

        let route_ctx = solution_ctx.routes.first().unwrap();
        assert_eq!(route_ctx.route().tour.get(1).unwrap().schedule, Schedule { arrival: 10., departure: 25. });
        assert_eq!(route_ctx.route().tour.get(2).unwrap().schedule, Schedule { arrival: 35., departure: 60. });
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .build();
        let target = Box::new(Activity {
            place: Place { idx: 0, location: 5, duration: 1.0, time: DEFAULT_ACTIVITY_TIME_WINDOW },
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

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &activity_ctx,
        ));

        assert_eq!(result, 21.0);
    }

    #[test]
    fn can_calculate_soft_activity_cost_for_non_empty_tour() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver_with_costs(empty_costs()))
            .add_vehicles(vec![TestVehicleBuilder::default().id("v1").build()])
            .build();
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(
                        ActivityBuilder::default()
                            .place(Place {
                                idx: 0,
                                location: 10,
                                duration: 0.0,
                                time: DEFAULT_ACTIVITY_TIME_WINDOW.clone(),
                            })
                            .schedule(Schedule { arrival: 0.0, departure: 10.0 })
                            .build(),
                    )
                    .add_activity(
                        ActivityBuilder::default()
                            .place(Place {
                                idx: 0,
                                location: 20,
                                duration: 0.0,
                                time: TimeWindow { start: 40.0, end: 70.0 },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build();
        let target = Box::new(Activity {
            place: Place { idx: 0, location: 30, duration: 10.0, time: DEFAULT_ACTIVITY_TIME_WINDOW },
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

        let result = create_feature().objective.unwrap().estimate(&MoveContext::activity(
            &solution_ctx,
            &route_ctx,
            &activity_ctx,
        ));

        assert_eq!(result, 30.0);
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
        let job = TestSingleBuilder::default().times(vec![TimeWindow::new(2000., 3000.)]).build_as_job_ref();

        let result =
            create_feature().constraint.unwrap().evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

        assert_eq!(result, ConstraintViolation::fail(VIOLATION_CODE));
    }
}

mod overtime {
    use super::*;
    use crate::construction::enablers::{TotalDistanceTourState, TotalDurationTourState};
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::models::problem::{OvertimeRateDimension, RegularDurationDimension, Vehicle};

    /// The point beyond which the shift is no longer paid at the regular rate.
    const REGULAR_DURATION: Duration = 3600.;
    /// Three times the vehicle's `per_driving_time`, which is what the premium is measured against
    /// and is 1 in `DEFAULT_VEHICLE_COSTS` - so the premium is 2. The tour is charged more than that
    /// per second: this fleet's `test_driver()` carries the same rates as the vehicle, so a regular
    /// second costs 2, once on each. The premium does not net that second rate off, by design.
    const OVERTIME_RATE: Cost = 3.;
    const PREMIUM: Cost = OVERTIME_RATE - DEFAULT_VEHICLE_COSTS.per_driving_time;

    fn create_feature() -> Feature {
        TransportFeatureBuilder::new("transport")
            .set_violation_code(VIOLATION_CODE)
            .set_transport_cost(TestTransportCost::new_shared())
            .set_activity_cost(TestActivityCost::new_shared())
            .build_minimize_cost()
            .unwrap()
    }

    /// A vehicle whose shift states its regular duration and overtime rate, or neither of them.
    fn create_vehicle(overtime_rate: Option<Cost>) -> Vehicle {
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id("v1");

        if let Some(overtime_rate) = overtime_rate {
            vehicle_builder.dimens_mut().set_overtime_rate(overtime_rate).set_regular_duration(REGULAR_DURATION);
        }

        vehicle_builder.build()
    }

    fn create_fleet(overtime_rate: Option<Cost>) -> Fleet {
        FleetBuilder::default().add_driver(test_driver()).add_vehicle(create_vehicle(overtime_rate)).build()
    }

    fn get_fitness(overtime_rate: Option<Cost>, total_duration: Duration) -> Cost {
        let fleet = create_fleet(overtime_rate);
        let mut state = RouteState::default();
        state.set_total_distance(100.);
        state.set_total_duration(total_duration);
        let route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build())
            .with_state(state)
            .build();
        let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

        create_feature().objective.unwrap().fitness(&insertion_ctx)
    }

    parameterized_test! {can_price_overtime_in_fitness, (total_duration, overtime_rate, expected), {
        can_price_overtime_in_fitness_impl(total_duration, overtime_rate, expected);
    }}

    can_price_overtime_in_fitness! {
        case01_above_the_threshold: (5400., OVERTIME_RATE, PREMIUM * 1800.),
        case02_at_the_threshold: (REGULAR_DURATION, OVERTIME_RATE, 0.),
        case03_below_the_threshold: (3000., OVERTIME_RATE, 0.),
        case04_cheaper_than_the_regular_rate: (5400., 0.5, 0.),
    }

    fn can_price_overtime_in_fitness_impl(total_duration: Duration, overtime_rate: Cost, expected: Cost) {
        let with_overtime = get_fitness(Some(overtime_rate), total_duration);
        let without_overtime = get_fitness(None, total_duration);

        assert_eq!(with_overtime - without_overtime, expected);
    }

    /// Estimates the same insertion twice: the target sits 100 units out from a tour which never
    /// leaves its depot, so it adds 100 there and 100 back - 200 seconds - to whatever the tour
    /// already runs.
    fn get_estimate(overtime_rate: Option<Cost>, total_duration: Duration) -> Cost {
        let fleet = create_fleet(overtime_rate);
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let mut state = RouteState::default();
        state.set_total_duration(total_duration);
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(ActivityBuilder::with_location(0).build())
                    .build(),
            )
            .with_state(state)
            .build();
        let target = ActivityBuilder::with_location(100).build();
        let activity_ctx = ActivityContext {
            index: 1,
            prev: route_ctx.route().tour.get(1).unwrap(),
            target: &target,
            next: route_ctx.route().tour.get(2),
        };

        create_feature().objective.unwrap().estimate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx))
    }

    parameterized_test! {can_price_overtime_in_estimate, (total_duration, expected), {
        can_price_overtime_in_estimate_impl(total_duration, expected);
    }}

    can_price_overtime_in_estimate! {
        case01_pushed_over_the_threshold: (3500., PREMIUM * 100.),
        case02_kept_under_the_threshold: (3000., 0.),
        case03_already_over_the_threshold: (4000., PREMIUM * 200.),
    }

    fn can_price_overtime_in_estimate_impl(total_duration: Duration, expected: Cost) {
        let with_overtime = get_estimate(Some(OVERTIME_RATE), total_duration);
        let without_overtime = get_estimate(None, total_duration);

        assert_eq!(with_overtime - without_overtime, expected);
    }
}

mod off_hours {
    use super::*;
    use crate::construction::enablers::get_paid_span;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::models::problem::{
        OffHoursRateDimension, RegularHours, RegularHoursDimension, RouteCostSpan, RouteCostSpanDimension,
    };
    use crate::models::solution::Route;

    const HOUR: Timestamp = 3600.;
    /// Twice the vehicle's `per_driving_time` (1 in `DEFAULT_VEHICLE_COSTS`), so the premium is 1 per second.
    const OFF_HOURS_RATE: Cost = 2.;
    const PREMIUM: Cost = OFF_HOURS_RATE - DEFAULT_VEHICLE_COSTS.per_driving_time;

    /// Regular hours 08:30 - 17:30.
    fn regular() -> RegularHours {
        RegularHours { earliest: 8.5 * HOUR, latest: 17.5 * HOUR }
    }

    fn create_fleet(hours: Option<RegularHours>, rate: Option<Cost>, span: Option<RouteCostSpan>) -> Fleet {
        let mut vehicle_builder = TestVehicleBuilder::default();
        vehicle_builder.id("v1");

        if let Some(hours) = hours {
            vehicle_builder.dimens_mut().set_regular_hours(hours);
        }
        if let Some(rate) = rate {
            vehicle_builder.dimens_mut().set_off_hours_rate(rate);
        }
        if let Some(span) = span {
            vehicle_builder.dimens_mut().set_route_cost_span(span);
        }

        FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle_builder.build()).build()
    }

    fn at(location: Location, arrival: Timestamp, departure: Timestamp) -> Activity {
        ActivityBuilder::with_location(location).schedule(Schedule::new(arrival, departure)).job(None).build()
    }

    /// Leaves home at `leave`, serves one job from `first` to `last`, is home at `home`.
    fn create_route(fleet: &Fleet, leave: Timestamp, first: Timestamp, last: Timestamp, home: Timestamp) -> Route {
        let job = ActivityBuilder::with_location(10)
            .schedule(Schedule::new(first, last))
            .job(Some(TestSingleBuilder::default().build_shared()))
            .build();

        RouteBuilder::default()
            .with_vehicle(fleet, "v1")
            .with_start(at(0, leave, leave))
            .with_end(at(0, home, home))
            .add_activities(vec![job])
            .build()
    }

    fn premium(leave: Timestamp, home: Timestamp, hours: Option<RegularHours>, rate: Option<Cost>) -> Cost {
        let fleet = create_fleet(hours, rate, None);
        let route = create_route(&fleet, leave, leave + 600., home - 600., home);

        get_off_hours_premium(route.actor.as_ref(), get_paid_span(&route).unwrap())
    }

    parameterized_test! {can_price_the_time_outside_the_regular_hours, (leave, home, expected), {
        assert_eq!(premium(leave, home, Some(regular()), Some(OFF_HOURS_RATE)), expected);
    }}

    can_price_the_time_outside_the_regular_hours! {
        case01_inside: (9. * HOUR, 17. * HOUR, 0.),
        case02_early: (7.5 * HOUR, 17. * HOUR, PREMIUM * HOUR),
        case03_both_sides: (8. * HOUR, 18.5 * HOUR, PREMIUM * (0.5 * HOUR + HOUR)),
        case04_exactly_the_regular_hours: (8.5 * HOUR, 17.5 * HOUR, 0.),
    }

    #[test]
    fn charges_nothing_without_regular_hours_or_rate_or_below_the_regular_rate() {
        assert_eq!(premium(6. * HOUR, 20. * HOUR, None, Some(OFF_HOURS_RATE)), 0.);
        assert_eq!(premium(6. * HOUR, 20. * HOUR, Some(regular()), None), 0.);
        assert_eq!(premium(6. * HOUR, 20. * HOUR, Some(regular()), Some(0.5)), 0.);
    }

    parameterized_test! {measures_the_paid_span, (span, expected), {
        let fleet = create_fleet(Some(regular()), Some(OFF_HOURS_RATE), Some(span));
        // Leaves 07:30, first visit 08:30, last visit ends 17:00, home 18:00.
        let route = create_route(&fleet, 7.5 * HOUR, 8.5 * HOUR, 17. * HOUR, 18. * HOUR);

        assert_eq!(get_off_hours_premium(route.actor.as_ref(), get_paid_span(&route).unwrap()), expected);
    }}

    measures_the_paid_span! {
        case01_depot_to_depot: (RouteCostSpan::DepotToDepot, PREMIUM * (HOUR + 0.5 * HOUR)),
        case02_depot_to_last_job: (RouteCostSpan::DepotToLastJob, PREMIUM * HOUR),
        case03_first_job_to_depot: (RouteCostSpan::FirstJobToDepot, PREMIUM * 0.5 * HOUR),
        case04_first_job_to_last_job: (RouteCostSpan::FirstJobToLastJob, 0.),
    }

    fn create_feature() -> Feature {
        TransportFeatureBuilder::new("transport")
            .set_violation_code(VIOLATION_CODE)
            .set_transport_cost(TestTransportCost::new_shared())
            .set_activity_cost(TestActivityCost::new_shared())
            .build_minimize_cost()
            .unwrap()
    }

    fn fitness(rate: Option<Cost>) -> Cost {
        let fleet = create_fleet(Some(regular()), rate, None);
        let route_ctx = RouteContextBuilder::default()
            .with_route(create_route(&fleet, 7.5 * HOUR, 8.5 * HOUR, 17. * HOUR, 17.5 * HOUR))
            .build();
        let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route_ctx]).build();

        create_feature().objective.unwrap().fitness(&insertion_ctx)
    }

    /// Estimates inserting a visit 100 units out between the last job and home: it pushes the end of
    /// the tour back by 200 seconds, as in the overtime estimate.
    fn estimate(rate: Option<Cost>, home: Timestamp) -> Cost {
        let fleet = create_fleet(Some(regular()), rate, None);
        let job = ActivityBuilder::with_location(0)
            .schedule(Schedule::new(9. * HOUR, home))
            .job(Some(TestSingleBuilder::default().build_shared()))
            .build();
        let route = RouteBuilder::default()
            .with_vehicle(&fleet, "v1")
            .with_start(at(0, 9. * HOUR, 9. * HOUR))
            .with_end(at(0, home, home))
            .add_activities(vec![job])
            .build();
        let route_ctx = RouteContextBuilder::default().with_route(route).build();
        let solution_ctx = TestInsertionContextBuilder::default().build().solution;
        let target = ActivityBuilder::with_location(100).build();
        let activity_ctx = ActivityContext {
            index: 1,
            prev: route_ctx.route().tour.get(1).unwrap(),
            target: &target,
            next: route_ctx.route().tour.get(2),
        };

        create_feature().objective.unwrap().estimate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx))
    }

    parameterized_test! {estimates_what_an_insertion_adds_after_the_regular_hours, (home, expected), {
        assert_eq!(estimate(Some(OFF_HOURS_RATE), home) - estimate(None, home), expected);
    }}

    estimates_what_an_insertion_adds_after_the_regular_hours! {
        case01_pushed_past_the_end: (17.5 * HOUR - 100., PREMIUM * 100.),
        case02_kept_inside: (17. * HOUR, 0.),
        case03_already_past_the_end: (18. * HOUR, PREMIUM * 200.),
    }

    #[test]
    fn adds_the_premium_to_the_cost_of_the_solution() {
        assert_eq!(fitness(Some(OFF_HOURS_RATE)) - fitness(None), PREMIUM * HOUR);
    }
}
