use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::TimeWindow;
use crate::models::problem::{JobIdDimension, SimpleActivityCost, Single, VisitWindow, VisitWindowKindDimension};
use rosomaxa::prelude::UnwrapValue;

fn route(recurring: Option<(Timestamp, Timestamp)>, other: Option<(Timestamp, Timestamp)>, overflow: bool) -> Route {
    let window = |(earliest, latest): (Timestamp, Timestamp)| VisitWindow { earliest, latest };
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_visit_windows(VisitWindows {
        recurring: recurring.map(window),
        other: other.map(window),
        overflow,
    });

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteBuilder::default().with_vehicle(&fleet, "v1").build()
}

fn route_without_windows() -> Route {
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build();

    RouteBuilder::default().with_vehicle(&fleet, "v1").build()
}

/// Treats every job as a visit; `breaks_are_never_bound` covers the other side.
fn cost() -> VisitWindowsActivityCost {
    VisitWindowsActivityCost::new(Arc::new(SimpleActivityCost::default()), Arc::new(|_| true))
}

fn activity(kind: Option<VisitWindowKind>, tw: (Timestamp, Timestamp), duration: Duration) -> Activity {
    let mut single = TestSingleBuilder::default();
    single.id("job1");
    if let Some(kind) = kind {
        single.dimens_mut().set_visit_window_kind(kind);
    }

    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(tw.0, tw.1), duration)
        .job(Some(single.build_shared()))
        .build()
}

#[test]
fn waits_for_the_recurring_window_to_open() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 10.),
        50.,
    );

    assert_eq!(departure.unwrap_value(), 110.);
}

#[test]
fn refuses_a_recurring_visit_that_ends_after_its_window() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        380.,
    );

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn overflow_lets_a_recurring_visit_use_the_other_window() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), Some((100., 800.)), true),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        600.,
    );

    assert_eq!(departure.unwrap_value(), 630.);
}

#[test]
fn overflow_without_other_window_is_no_overflow() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, true),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        600.,
    );

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn an_other_visit_keeps_to_the_other_window() {
    let route = route(Some((100., 400.)), Some((100., 800.)), false);

    let inside = cost().estimate_departure(&route, &activity(Some(VisitWindowKind::Other), (0., 1000.), 30.), 600.);
    let after = cost().estimate_departure(&route, &activity(Some(VisitWindowKind::Other), (0., 1000.), 30.), 790.);

    assert_eq!(inside.unwrap_value(), 630.);
    assert!(matches!(after, ControlFlow::Break(_)));
}

#[test]
fn a_fixed_visit_is_never_bound() {
    let departure =
        cost().estimate_departure(&route(Some((100., 400.)), None, false), &activity(None, (900., 1000.), 30.), 900.);

    assert_eq!(departure.unwrap_value(), 930.);
}

#[test]
fn untagged_shift_leaves_the_job_alone() {
    let departure = cost().estimate_departure(
        &route_without_windows(),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        900.,
    );

    assert_eq!(departure.unwrap_value(), 930.);
}

#[test]
fn refuses_when_raised_arrival_passes_the_job_window() {
    let departure = cost().estimate_departure(
        &route(Some((500., 800.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 300.), 30.),
        50.,
    );

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn breaks_are_never_bound() {
    let is_visit: IsAppointmentFn =
        Arc::new(|single: &Single| !matches!(single.dimens.get_job_id().map(String::as_str), Some("job1")));
    let cost = VisitWindowsActivityCost::new(Arc::new(SimpleActivityCost::default()), is_visit);

    let departure = cost.estimate_departure(
        &route(Some((100., 400.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        900.,
    );

    assert_eq!(departure.unwrap_value(), 930.);
}

#[test]
fn caps_the_backward_pass_at_the_window_end() {
    let arrival = cost().estimate_arrival(
        &route(Some((100., 400.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 30.),
        900.,
    );

    assert_eq!(arrival.unwrap_value(), 370.);
}

#[test]
fn reports_the_service_start_the_window_forces() {
    let service_start = cost().estimate_service_start(
        &route(Some((100., 400.)), None, false),
        &activity(Some(VisitWindowKind::Recurring), (0., 1000.), 10.),
        50.,
    );

    assert_eq!(service_start, 100.);
}

#[test]
fn overflow_waits_for_the_own_window_when_the_visit_fits_it() {
    let route = route(Some((50., 70.)), Some((0., 100.)), true);
    let activity = activity(Some(VisitWindowKind::Recurring), (0., 1000.), 10.);

    assert_eq!(cost().estimate_departure(&route, &activity, 1.).unwrap_value(), 60.);
    assert_eq!(cost().estimate_service_start(&route, &activity, 1.), 50.);
}

#[test]
fn overflow_serves_at_once_when_the_own_window_has_passed() {
    let route = route(Some((50., 70.)), Some((0., 100.)), true);
    let activity = activity(Some(VisitWindowKind::Recurring), (0., 1000.), 10.);

    assert_eq!(cost().estimate_departure(&route, &activity, 65.).unwrap_value(), 75.);
    assert_eq!(cost().estimate_service_start(&route, &activity, 65.), 65.);
}
