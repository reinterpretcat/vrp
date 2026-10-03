use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::TimeWindow;
use crate::models::problem::{JobIdDimension, SimpleActivityCost, Single, VisitWindow, VisitWindowTagDimension};
use rosomaxa::prelude::UnwrapValue;

/// A shift with an `own` window that falls back to a `fallback` window when `overflow` is on.
fn route(own: Option<(Timestamp, Timestamp)>, fallback: Option<(Timestamp, Timestamp)>, overflow: bool) -> Route {
    let window = |(earliest, latest): (Timestamp, Timestamp), next: Option<&str>| VisitWindow {
        earliest,
        latest,
        fallback: next.map(str::to_string),
        bridge: false,
    };
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_visit_windows(VisitWindows {
        windows: own
            .map(|own| ("own".to_string(), window(own, overflow.then_some("fallback"))))
            .into_iter()
            .chain(fallback.map(|fallback| ("fallback".to_string(), window(fallback, None))))
            .collect(),
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

fn activity(tag: Option<&str>, tw: (Timestamp, Timestamp), duration: Duration) -> Activity {
    let mut single = TestSingleBuilder::default();
    single.id("job1");
    if let Some(tag) = tag {
        single.dimens_mut().set_visit_window_tag(tag.to_string());
    }

    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(tw.0, tw.1), duration)
        .job(Some(single.build_shared()))
        .build()
}

#[test]
fn waits_for_the_own_window_to_open() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, false),
        &activity(Some("own"), (0., 1000.), 10.),
        50.,
    );

    assert_eq!(departure.unwrap_value(), 110.);
}

#[test]
fn refuses_a_visit_that_ends_after_its_window() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, false),
        &activity(Some("own"), (0., 1000.), 30.),
        380.,
    );

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn overflow_lets_a_visit_use_its_fallback_window() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), Some((100., 800.)), true),
        &activity(Some("own"), (0., 1000.), 30.),
        600.,
    );

    assert_eq!(departure.unwrap_value(), 630.);
}

#[test]
fn a_fallback_the_shift_lacks_is_no_overflow() {
    let departure = cost().estimate_departure(
        &route(Some((100., 400.)), None, true),
        &activity(Some("own"), (0., 1000.), 30.),
        600.,
    );

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn a_visit_tagged_with_the_fallback_keeps_to_it() {
    let route = route(Some((100., 400.)), Some((100., 800.)), false);

    let inside = cost().estimate_departure(&route, &activity(Some("fallback"), (0., 1000.), 30.), 600.);
    let after = cost().estimate_departure(&route, &activity(Some("fallback"), (0., 1000.), 30.), 790.);

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
    let departure = cost().estimate_departure(&route_without_windows(), &activity(Some("own"), (0., 1000.), 30.), 900.);

    assert_eq!(departure.unwrap_value(), 930.);
}

#[test]
fn refuses_when_raised_arrival_passes_the_job_window() {
    let departure = cost().estimate_departure(
        &route(Some((500., 800.)), None, false),
        &activity(Some("own"), (0., 300.), 30.),
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
        &activity(Some("own"), (0., 1000.), 30.),
        900.,
    );

    assert_eq!(departure.unwrap_value(), 930.);
}

#[test]
fn caps_the_backward_pass_at_the_window_end() {
    let arrival = cost().estimate_arrival(
        &route(Some((100., 400.)), None, false),
        &activity(Some("own"), (0., 1000.), 30.),
        900.,
    );

    assert_eq!(arrival.unwrap_value(), 370.);
}

#[test]
fn reports_the_service_start_the_window_forces() {
    let service_start = cost().estimate_service_start(
        &route(Some((100., 400.)), None, false),
        &activity(Some("own"), (0., 1000.), 10.),
        50.,
    );

    assert_eq!(service_start, 100.);
}

#[test]
fn overflow_waits_for_the_own_window_when_the_visit_fits_it() {
    let route = route(Some((50., 70.)), Some((0., 100.)), true);
    let activity = activity(Some("own"), (0., 1000.), 10.);

    assert_eq!(cost().estimate_departure(&route, &activity, 1.).unwrap_value(), 60.);
    assert_eq!(cost().estimate_service_start(&route, &activity, 1.), 50.);
}

#[test]
fn overflow_serves_at_once_when_the_own_window_has_passed() {
    let route = route(Some((50., 70.)), Some((0., 100.)), true);
    let activity = activity(Some("own"), (0., 1000.), 10.);

    assert_eq!(cost().estimate_departure(&route, &activity, 65.).unwrap_value(), 75.);
    assert_eq!(cost().estimate_service_start(&route, &activity, 65.), 65.);
}

/// A shift whose `own` window 50–100 is bridged, holding `fixed` untagged visits with these windows.
fn bridged_route(fixed: Vec<(Timestamp, Timestamp)>) -> Route {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_visit_windows(VisitWindows {
        windows: [("own".to_string(), VisitWindow { earliest: 50., latest: 100., fallback: None, bridge: true })]
            .into_iter()
            .collect(),
    });
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteBuilder::default()
        .with_vehicle(&fleet, "v1")
        .add_activities(fixed.into_iter().map(|tw| activity(None, tw, 5.)))
        .build()
}

#[test]
fn a_bridged_window_reaches_out_to_an_untagged_visit_on_the_route() {
    let departure =
        cost().estimate_departure(&bridged_route(vec![(20., 30.)]), &activity(Some("own"), (0., 1000.), 10.), 31.);

    assert_eq!(departure, ControlFlow::Continue(41.));
}

#[test]
fn a_bridged_window_reaches_out_after_the_window_too() {
    let departure =
        cost().estimate_departure(&bridged_route(vec![(150., 160.)]), &activity(Some("own"), (0., 1000.), 10.), 120.);

    assert_eq!(departure, ControlFlow::Continue(130.));
}

#[test]
fn a_bridged_window_without_untagged_visits_is_the_window_itself() {
    let departure = cost().estimate_departure(&bridged_route(vec![]), &activity(Some("own"), (0., 1000.), 10.), 31.);

    assert_eq!(departure, ControlFlow::Continue(60.));
}

#[test]
fn a_fallback_need_not_contain_the_own_window() {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_visit_windows(VisitWindows {
        windows: [
            (
                "morning".to_string(),
                VisitWindow { earliest: 8., latest: 12., fallback: Some("evening".into()), bridge: false },
            ),
            ("evening".to_string(), VisitWindow { earliest: 14., latest: 18., fallback: None, bridge: false }),
        ]
        .into_iter()
        .collect(),
    });
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();
    let route = RouteBuilder::default().with_vehicle(&fleet, "v1").build();

    let departure = cost().estimate_departure(&route, &activity(Some("morning"), (0., 1000.), 2.), 13.);

    assert_eq!(departure, ControlFlow::Continue(16.));
}
