use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::SimpleActivityCost;
use rosomaxa::prelude::UnwrapValue;

fn route_with_bounds(earliest_first: Option<Timestamp>, latest_last: Option<Timestamp>) -> Route {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_job_time_constraints(JobTimeConstraints { earliest_first, latest_last });

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteBuilder::default().with_vehicle(&fleet, "v1").build()
}

#[test]
fn waits_until_the_lower_bound_instead_of_serving_early() {
    let cost = JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()));
    let route = route_with_bounds(Some(100.), None);
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    let departure = cost.estimate_departure(&route, &activity, 50.);

    // served at the bound, not at arrival: 100 + 10
    assert_eq!(departure.unwrap_value(), 110.);
}

#[test]
fn refuses_a_departure_past_the_upper_bound() {
    let cost = JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()));
    let route = route_with_bounds(None, Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    let departure = cost.estimate_departure(&route, &activity, 95.);

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn caps_the_latest_departure_when_walking_backwards() {
    let cost = JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()));
    let route = route_with_bounds(None, Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    // asked for a departure at 500, the bound cuts it to 100 → arrival 90
    assert_eq!(cost.estimate_arrival(&route, &activity, 500.).unwrap_value(), 90.);
}

#[test]
fn leaves_a_non_job_activity_alone() {
    let cost = JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()));
    let route = route_with_bounds(Some(100.), Some(100.));
    let activity = ActivityBuilder::default().job(None).build(); // no job — a depot activity

    assert_eq!(cost.estimate_departure(&route, &activity, 50.).unwrap_value(), 50.);
}
