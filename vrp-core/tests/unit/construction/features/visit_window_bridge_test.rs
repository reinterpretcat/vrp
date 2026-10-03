use super::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Schedule, TimeWindow};
use crate::models::problem::{Single, VisitWindow, VisitWindowTagDimension, VisitWindows, VisitWindowsDimension};
use crate::models::solution::Activity;

/// A visit served from `start` to `end`, tagged `own` or untagged, with its own time window `tw`.
fn visit(id: &str, tag: Option<&str>, tw: (Timestamp, Timestamp), start: Timestamp, end: Timestamp) -> Activity {
    let mut single = TestSingleBuilder::default();
    single.id(id).duration(end - start);
    if let Some(tag) = tag {
        single.dimens_mut().set_visit_window_tag(tag.to_string());
    }

    ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(tw.0, tw.1), end - start)
        .schedule(Schedule::new(start, end))
        .job(Some(single.build_shared()))
        .build()
}

/// A route on a shift whose `own` window 50–100 is bridged.
fn route_with(activities: Vec<Activity>) -> RouteContext {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_visit_windows(VisitWindows {
        windows: [("own".to_string(), VisitWindow { earliest: 50., latest: 100., fallback: None, bridge: true })]
            .into_iter()
            .collect(),
    });
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build()
}

fn feature() -> Feature {
    create_visit_window_bridge_feature("bridge", Arc::new(|_: &Single| true)).unwrap()
}

#[test]
fn returns_bridged_visits_once_the_untagged_visit_is_gone() {
    // the regular visit was served 31–41, next to an untagged visit at 20–30 that ruin removed.
    let mut solution = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with(vec![visit("regular", Some("own"), (0., 1000.), 31., 41.)])])
        .build()
        .solution;

    feature().state.unwrap().accept_solution_state(&mut solution);

    assert_eq!(solution.routes[0].route().tour.job_count(), 0);
    assert_eq!(solution.unassigned.len(), 1);
}

#[test]
fn keeps_bridged_visits_while_the_untagged_visit_stays() {
    let mut solution = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with(vec![
            visit("fixed", None, (20., 30.), 20., 25.),
            visit("regular", Some("own"), (0., 1000.), 31., 41.),
        ])])
        .build()
        .solution;

    feature().state.unwrap().accept_solution_state(&mut solution);

    assert_eq!(solution.routes[0].route().tour.job_count(), 2);
    assert!(solution.unassigned.is_empty());
}

#[test]
fn keeps_visits_inside_their_own_window() {
    let mut solution = TestInsertionContextBuilder::default()
        .with_routes(vec![route_with(vec![visit("regular", Some("own"), (0., 1000.), 60., 70.)])])
        .build()
        .solution;

    feature().state.unwrap().accept_solution_state(&mut solution);

    assert_eq!(solution.routes[0].route().tour.job_count(), 1);
}
