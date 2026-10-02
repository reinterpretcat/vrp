use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::solution::Activity;

fn windows() -> VisitWindows {
    VisitWindows {
        recurring: Some(VisitWindow { earliest: 0., latest: 20. }),
        other: Some(VisitWindow { earliest: 0., latest: 100. }),
        overflow: true,
    }
}

fn single(kind: Option<VisitWindowKind>) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();
    builder.duration(10.);
    if let Some(kind) = kind {
        builder.dimens_mut().set_visit_window_kind(kind);
    }
    builder.build_shared()
}

fn visit(kind: Option<VisitWindowKind>, arrival: Timestamp, departure: Timestamp) -> Activity {
    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.)
        .schedule(Schedule::new(arrival, departure))
        .job(Some(single(kind)))
        .build()
}

fn route_with(windows: Option<VisitWindows>, activities: Vec<Activity>) -> RouteContext {
    let mut vehicle = test_vehicle_with_id("v1");
    if let Some(windows) = windows {
        vehicle.dimens.set_visit_windows(windows);
    }
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build()
}

fn feature() -> Feature {
    create_visit_window_overflow_feature(
        "overflow",
        TestTransportCost::new_shared(),
        Arc::new(SimpleActivityCost::default()),
    )
    .unwrap()
}

#[test]
fn counts_recurring_visits_outside_their_window() {
    let route = route_with(
        Some(windows()),
        vec![
            visit(Some(VisitWindowKind::Recurring), 5., 15.),
            visit(Some(VisitWindowKind::Recurring), 50., 60.),
            visit(Some(VisitWindowKind::Other), 70., 80.),
            visit(None, 90., 100.),
        ],
    );
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route]).build();

    assert_eq!(feature().objective.unwrap().fitness(&insertion_ctx), 1.);
}

#[test]
fn counts_nothing_on_a_shift_without_windows() {
    let route = route_with(None, vec![visit(Some(VisitWindowKind::Recurring), 50., 60.)]);
    let insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![route]).build();

    assert_eq!(feature().objective.unwrap().fitness(&insertion_ctx), 0.);
}

#[test]
fn estimates_one_for_a_recurring_visit_that_would_overflow() {
    let route = route_with(Some(windows()), vec![visit(None, 40., 50.)]);
    let objective = feature().objective.unwrap();
    let prev = route.route().tour.get(1).unwrap();

    let solution_ctx = TestInsertionContextBuilder::default().build().solution;
    let estimate = |kind: VisitWindowKind| {
        let target = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.)
            .job(Some(single(Some(kind))))
            .build();
        let activity_ctx = ActivityContext { index: 1, prev, target: &target, next: None };
        objective.estimate(&MoveContext::activity(&solution_ctx, &route, &activity_ctx))
    };

    assert_eq!(estimate(VisitWindowKind::Recurring), 1.);
    assert_eq!(estimate(VisitWindowKind::Other), 0.);
}
