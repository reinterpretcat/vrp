use super::*;
use crate::construction::enablers::OnlyVehicleActivityCost;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::{JobIdDimension, SimpleActivityCost};
use rosomaxa::prelude::UnwrapValue;

fn route_with_bounds(earliest_first: Option<Timestamp>, latest_last: Option<Timestamp>) -> Route {
    let mut vehicle = test_vehicle_with_id("v1");
    vehicle.dimens.set_job_time_constraints(JobTimeConstraints { earliest_first, latest_last });

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    RouteBuilder::default().with_vehicle(&fleet, "v1").build()
}

/// Treats every job as an appointment. Which ones really are is the format's business, and
/// `leaves_an_activity_the_predicate_rejects_alone` covers the other side of that.
fn bounded_cost() -> JobTimeBoundsActivityCost {
    JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()), Arc::new(|_| true))
}

#[test]
fn waits_until_the_lower_bound_instead_of_serving_early() {
    let cost = bounded_cost();
    let route = route_with_bounds(Some(100.), None);
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    let departure = cost.estimate_departure(&route, &activity, 50.);

    // served at the bound, not at arrival: 100 + 10
    assert_eq!(departure.unwrap_value(), 110.);
}

#[test]
fn refuses_a_departure_past_the_upper_bound() {
    let cost = bounded_cost();
    let route = route_with_bounds(None, Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    let departure = cost.estimate_departure(&route, &activity, 95.);

    assert!(matches!(departure, ControlFlow::Break(_)));
}

#[test]
fn caps_the_latest_departure_when_walking_backwards() {
    let cost = bounded_cost();
    let route = route_with_bounds(None, Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    // asked for a departure at 500, the bound cuts it to 100 → arrival 90
    assert_eq!(cost.estimate_arrival(&route, &activity, 500.).unwrap_value(), 90.);
}

#[test]
fn leaves_a_non_job_activity_alone() {
    let cost = bounded_cost();
    let route = route_with_bounds(Some(100.), Some(100.));
    let activity = ActivityBuilder::default().job(None).build(); // no job — a depot activity

    assert_eq!(cost.estimate_departure(&route, &activity, 50.).unwrap_value(), 50.);
}

#[test]
fn leaves_an_activity_the_predicate_rejects_alone() {
    // a break is a job on the tour, but it is not an appointment: the format says so through the
    // predicate, the same way `is_stop` does for the pragmatic format.
    let is_appointment: IsAppointmentFn =
        Arc::new(|single: &Single| !matches!(single.dimens.get_job_id().map(String::as_str), Some("break")));
    let cost = JobTimeBoundsActivityCost::new(Arc::new(SimpleActivityCost::default()), is_appointment);
    let route = route_with_bounds(Some(100.), Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.)
        .job(Some(TestSingleBuilder::default().id("break").build_shared()))
        .build();

    let departure = cost.estimate_departure(&route, &activity, 50.);

    // an appointment here would be held back to 100 and then refused for departing past 100;
    // the break is neither, so it is served on arrival: 50 + 10
    assert!(matches!(departure, ControlFlow::Continue(_)), "a break must not be refused by the bounds");
    assert_eq!(departure.unwrap_value(), 60.);
}

#[test]
fn charges_what_the_inner_cost_charges() {
    let inner = Arc::new(OnlyVehicleActivityCost::default());
    let cost = JobTimeBoundsActivityCost::new(inner.clone(), Arc::new(|_| true));
    let route = route_with_bounds(Some(100.), Some(100.));
    let activity = ActivityBuilder::with_location_tw_and_duration(1, TimeWindow::new(0., 1000.), 10.).build();

    // no waiting at arrival 50, so this is ten units of service at the vehicle's rate. The trait's
    // default would answer 20: it adds the driver's share, which the inner cost drops on purpose.
    assert_eq!(cost.cost(&route, &activity, 50.), 10.);
    assert_eq!(cost.cost(&route, &activity, 50.), inner.cost(&route, &activity, 50.));
}
