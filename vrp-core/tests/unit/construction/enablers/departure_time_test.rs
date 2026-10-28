use super::*;
use crate::construction::enablers::{DynamicActivityCost, DynamicTransportCost, ReservedTimeSpan};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::*;
use crate::models::problem::Place as JobPlace;
use crate::models::problem::*;
use crate::models::solution::{Activity, Place as ActivityPlace};
use rosomaxa::prelude::Float;
use std::sync::Arc;

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
    latest: Option<Float>,
    optimize_whole_tour: bool,
    tws: Vec<TimeWindow>,
    expected: Option<Float>,
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
        let route_ctx = RouteContextBuilder::default()
            .with_route(
                RouteBuilder::default()
                    .with_vehicle(&fleet, "v1")
                    .add_activity(ActivityBuilder::with_location_and_tw(10, tw1.clone()).build())
                    .add_activity(ActivityBuilder::with_location_and_tw(20, tw2.clone()).build())
                    .add_activity(ActivityBuilder::with_location_and_tw(30, tw3.clone()).build())
                    .build(),
            )
            .build();

        let departure_time = try_advance_departure_time(
            &route_ctx,
            &SimpleActivityCost::default(),
            &TestTransportCost::default(),
            optimize_whole_tour,
        );

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
    earliest: Option<Float>,
    start_departure: Float,
    latest_first_arrival: Float,
    tw: TimeWindow,
    total_duration_limit: Option<(Float, Float)>,
    expected: Option<Float>,
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
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activity(ActivityBuilder::with_location_and_tw(10, tw).build())
                .build(),
        )
        .build();
    let (route, state) = route_ctx.as_mut();
    route.tour.get_mut(0).unwrap().schedule.departure = start_departure;
    state.set_schedule_states(vec![
        ActivityScheduleState::default(),
        ActivityScheduleState { latest_arrival: latest_first_arrival, ..ActivityScheduleState::default() },
    ]);

    if let Some((total, limit)) = total_duration_limit {
        state.set_total_duration(total);
        state.set_limit_duration(limit);
    }

    let departure_time = try_recede_departure_time(&route_ctx);

    assert_eq!(departure_time, expected);
}

#[test]
fn recomputes_offset_time_windows_on_departure_shift() {
    let offset = TimeOffset::new(10., 12.);
    let old_departure = 0.;
    let new_departure = 5.;

    let job = {
        let mut dimens = Dimensions::default();
        dimens.set_job_id("break".to_string());

        Arc::new(Single {
            places: vec![JobPlace { location: Some(1), duration: 0., times: vec![TimeSpan::Offset(offset.clone())] }],
            dimens,
        })
    };

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&test_fleet(), "v1")
                .add_activity({
                    let mut activity = Activity::new_with_job(job.clone());
                    activity.place = ActivityPlace {
                        idx: 0,
                        location: job.places[0].location.unwrap(),
                        duration: job.places[0].duration,
                        time: TimeSpan::Offset(offset.clone()).to_time_window(old_departure),
                    };
                    activity
                })
                .build(),
        )
        .build();

    update_route_departure(&mut route_ctx, &TestActivityCost::default(), &TestTransportCost::default(), new_departure);

    let activity = route_ctx.route().tour.get(1).unwrap();
    assert_eq!(activity.place.time, TimeSpan::Offset(offset).to_time_window(new_departure));
}

/// A route can already sit above its duration limit - a supplied initial solution, or the one
/// `TravelLimitState::notify_failure` forces open - and a departure move that brings it back toward
/// the limit has to be kept even when it does not reach it. Weighing the limit absolutely would
/// refuse the move and leave the route on the longer tour.
///
/// Here the vehicle idles from 1 until the job's window opens at 100, works 200 seconds and is home
/// at 301. Departing at 99 instead takes that idle out: the tour is 202, still four times the 50s
/// limit, but 99 seconds shorter than the one the route has.
#[test]
fn can_keep_a_departure_that_shortens_a_route_already_past_its_limit() {
    let activity_cost = TestActivityCost::default();
    let transport_cost = TestTransportCost::default();

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&test_fleet(), "v1")
                .add_activity(
                    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(100., 1000.), 200.).build(),
                )
                .build(),
        )
        .build();
    update_route_schedule(&mut route_ctx, &activity_cost, &transport_cost);
    route_ctx.state_mut().set_limit_duration(50.);

    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(301.));

    advance_departure_time(&mut route_ctx, &activity_cost, &transport_cost, true);

    assert_eq!(route_ctx.route().tour.start().unwrap().schedule.departure, 99.);
    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(202.));
}

/// The same route under a limit the move does reach: nothing special, it is kept because it fits.
#[test]
fn can_keep_a_departure_that_brings_a_route_back_under_its_limit() {
    let activity_cost = TestActivityCost::default();
    let transport_cost = TestTransportCost::default();

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&test_fleet(), "v1")
                .add_activity(
                    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(100., 1000.), 200.).build(),
                )
                .build(),
        )
        .build();
    update_route_schedule(&mut route_ctx, &activity_cost, &transport_cost);
    route_ctx.state_mut().set_limit_duration(250.);

    advance_departure_time(&mut route_ctx, &activity_cost, &transport_cost, true);

    assert_eq!(route_ctx.route().tour.start().unwrap().schedule.departure, 99.);
    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(202.));
}

fn create_first_job_to_last_job_route(
    reserved_time: (Timestamp, Duration),
    jobs: Vec<(Location, (Timestamp, Timestamp), Duration)>,
) -> (DynamicActivityCost, DynamicTransportCost, RouteContext) {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_route_cost_span(RouteCostSpan::FirstJobToLastJob);
    vehicle.details = vec![VehicleDetail {
        start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0.), latest: None } }),
        end: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: None, latest: Some(1000.) } }),
    }];
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();
    let actor = fleet.actors.first().unwrap().clone();

    let (time, duration) = reserved_time;
    let reserved_times_index =
        vec![(actor, vec![ReservedTimeSpan { time: TimeSpan::Window(TimeWindow::new(time, time)), duration }])]
            .into_iter()
            .collect::<std::collections::HashMap<_, _>>();
    let activity_cost = DynamicActivityCost::new(reserved_times_index.clone()).unwrap();
    let transport_cost =
        DynamicTransportCost::new(reserved_times_index, Arc::new(TestTransportCost::default())).unwrap();

    let activities = jobs.into_iter().map(|(location, (start, end), duration)| {
        ActivityBuilder::with_location_tw_and_duration(location, TimeWindow::new(start, end), duration).build()
    });
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build();
    update_route_schedule(&mut route_ctx, &activity_cost, &transport_cost);

    (activity_cost, transport_cost, route_ctx)
}

/// A shift paid from its first job to its last, with a fixed reserved break, departing at its
/// `earliest` the way a route does right after a job went in front of its first one
/// (`apply_insertion_success`), or one taken from the registry.
///
/// Departing at 0 the vehicle idles at job A (window 100..200) from 10 to 100 and at job B (window
/// 300..400) from 120 to 300; the 10s break due at 205 falls into the idle at B and costs nothing.
/// The paid span runs from A's arrival at 10 to B's departure at 310: 300, past the 250 cap.
///
/// The whole-tour advance wants to depart at 190, which lands A's service on the break and pushes it
/// past A's window, so that departure is refused. Departing at 90 instead reaches A just as its
/// window opens, keeps the break in the idle at B and pays 210 - the departure the duration
/// constraint counts on when it accepts a job in front of the first one. Falling back to 0 instead
/// leaves the route 50 over a cap the constraint believed it honoured.
#[test]
fn can_fall_back_to_the_first_job_departure_when_the_whole_tour_advance_hits_a_break() {
    let (activity_cost, transport_cost, mut route_ctx) =
        create_first_job_to_last_job_route((205., 10.), vec![(10, (100., 200.), 10.), (20, (300., 400.), 10.)]);
    route_ctx.state_mut().set_limit_duration(250.);

    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(300.));

    advance_departure_time(&mut route_ctx, &activity_cost, &transport_cost, true);

    assert_eq!(route_ctx.route().tour.start().unwrap().schedule.departure, 90.);
    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(210.));
}

/// The production shape (2026-10-23, `shift time limit violation ... got: 59261`): a shift that
/// opens at midnight, a 20s break due at 95, and a first job whose window opens at 100.
///
/// Departing at 0 the break is spent idling at A, A is served 115..165 and B, which has to be
/// reached by 178, at 175. Leaving just in time for A, at 90, puts the break on the drive: A is
/// reached at 120 and B at 180, too late - and that is also where the whole-tour advance wants to
/// go. Every departure up to 88 still reaches B in time. Restoring 0 instead pays for the whole night
/// in front of A: 175 against a cap of 100, where leaving at 88 pays 70.
#[test]
fn can_find_a_departure_that_takes_a_break_before_the_first_job() {
    let (activity_cost, transport_cost, mut route_ctx) =
        create_first_job_to_last_job_route((95., 20.), vec![(10, (100., 400.), 50.), (20, (100., 178.), 10.)]);
    route_ctx.state_mut().set_limit_duration(100.);

    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(175.));

    advance_departure_time(&mut route_ctx, &activity_cost, &transport_cost, true);

    assert_eq!(route_ctx.route().tour.start().unwrap().schedule.departure, 88.);
    assert_eq!(route_ctx.state().get_total_duration().copied(), Some(70.));
}
