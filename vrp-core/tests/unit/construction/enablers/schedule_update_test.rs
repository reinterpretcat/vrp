use super::*;
use crate::construction::enablers::PaidWorkingDurationTourState;
use crate::construction::enablers::{
    DynamicActivityCost, DynamicTransportCost, ReservedTimeSpan, TotalDistanceTourState, TotalDurationTourState,
};
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Location, Schedule, TimeInterval, TimeOffset, TimeSpan, TimeWindow, Timestamp};
use crate::models::problem::{RouteCostSpan, RouteCostSpanDimension, VehicleDetail, VehiclePlace};
use std::sync::Arc;

fn create_detail(start_loc: Location, end_loc: Location) -> VehicleDetail {
    VehicleDetail {
        start: Some(VehiclePlace { location: start_loc, time: TimeInterval { earliest: Some(0.), latest: None } }),
        end: Some(VehiclePlace { location: end_loc, time: TimeInterval { earliest: None, latest: Some(1000.) } }),
    }
}

fn create_open_detail(start_loc: Location) -> VehicleDetail {
    VehicleDetail {
        start: Some(VehiclePlace { location: start_loc, time: TimeInterval { earliest: Some(0.), latest: None } }),
        end: None, // Open VRP - no end depot
    }
}

fn create_activity_with_location_and_schedule(
    location: Location,
    arrival: Timestamp,
    departure: Timestamp,
) -> Activity {
    let mut activity = ActivityBuilder::with_location(location).build();
    activity.schedule = Schedule::new(arrival, departure);
    activity
}

/// Creates a route with:
/// - Depot at location 0 (start and end)
/// - Job 1 at location 10
/// - Job 2 at location 30
/// - Job 3 at location 60
///
/// With TestTransportCost (distance = |to - from|):
/// - Depot(0) -> Job1(10): distance = 10
/// - Job1(10) -> Job2(30): distance = 20
/// - Job2(30) -> Job3(60): distance = 30
/// - Job3(60) -> Depot(0): distance = 60
///
/// Total distances by span:
/// - DepotToDepot: 10 + 20 + 30 + 60 = 120
/// - DepotToLastJob: 10 + 20 + 30 = 60
/// - FirstJobToDepot: 20 + 30 + 60 = 110
/// - FirstJobToLastJob: 20 + 30 = 50
fn create_test_route_with_cost_span(cost_span: Option<RouteCostSpan>) -> (RouteContext, TestTransportCost) {
    let mut vehicle = TestVehicleBuilder::default().id("v1").details(vec![create_detail(0, 0)]).build();

    if let Some(span) = cost_span {
        vehicle.dimens.set_route_cost_span(span);
    }

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    // Build route with start at 0, jobs at 10, 30, 60, end at 0
    // Schedules are set to reflect travel times (using location as arrival time for simplicity)
    let route = RouteBuilder::default()
        .with_vehicle(&fleet, "v1")
        .with_start({
            let mut start = ActivityBuilder::default().build();
            start.place.location = 0;
            start.schedule = Schedule::new(0., 0.);
            start.job = None;
            start
        })
        .with_end({
            let mut end = ActivityBuilder::default().build();
            end.place.location = 0;
            end.schedule = Schedule::new(130., 130.); // arrival after traveling back from 60
            end.job = None;
            end
        })
        .add_activities(vec![
            // Job 1 at location 10: arrive at 10 (0 + 10), depart at 10
            create_activity_with_location_and_schedule(10, 10., 10.),
            // Job 2 at location 30: arrive at 30 (10 + 20), depart at 30
            create_activity_with_location_and_schedule(30, 30., 30.),
            // Job 3 at location 60: arrive at 60 (30 + 30), depart at 60
            create_activity_with_location_and_schedule(60, 60., 60.),
        ])
        .build();

    let route_ctx = RouteContextBuilder::default().with_route(route).build();

    (route_ctx, TestTransportCost::default())
}

#[test]
fn can_calculate_statistics_with_depot_to_depot_span() {
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::DepotToDepot));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Distance: 0->10 + 10->30 + 30->60 + 60->0 = 10 + 20 + 30 + 60 = 120
    assert_eq!(total_distance, 120., "DepotToDepot distance should be 120");
    // Duration: end.departure(130) - start.departure(0) = 130
    assert_eq!(total_duration, 130., "DepotToDepot duration should be 130");
}

#[test]
fn can_calculate_statistics_with_depot_to_last_job_span() {
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::DepotToLastJob));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Distance is span-blind: the whole route, 0->10 + 10->30 + 30->60 + 60->0 = 120. The span
    // decides what the technician is PAID for, not which miles the vehicle drove.
    assert_eq!(total_distance, 120., "DepotToLastJob distance is still the whole route");
    // Duration: last_job.departure(60) - start.departure(0) = 60
    assert_eq!(total_duration, 60., "DepotToLastJob duration should be 60");
}

#[test]
fn can_calculate_statistics_with_first_job_to_depot_span() {
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::FirstJobToDepot));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Distance is span-blind: the whole route, including the outbound leg the span excludes.
    assert_eq!(total_distance, 120., "FirstJobToDepot distance is still the whole route");
    // Duration: end.departure(130) - first_job.arrival(10) = 120
    assert_eq!(total_duration, 120., "FirstJobToDepot duration should be 120");
}

#[test]
fn can_calculate_statistics_with_first_job_to_last_job_span() {
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::FirstJobToLastJob));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Distance is span-blind: the whole route, including both depot legs the span excludes.
    assert_eq!(total_distance, 120., "FirstJobToLastJob distance is still the whole route");
    // Duration: last_job.departure(60) - first_job.arrival(10) = 50
    assert_eq!(total_duration, 50., "FirstJobToLastJob duration should be 50");
}

#[test]
fn can_calculate_statistics_with_default_span_when_not_set() {
    // When no span is set, should default to DepotToDepot
    let (mut route_ctx, transport) = create_test_route_with_cost_span(None);

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Should match DepotToDepot behavior
    assert_eq!(total_distance, 120., "Default span distance should match DepotToDepot");
    assert_eq!(total_duration, 130., "Default span duration should match DepotToDepot");
}

#[test]
fn can_handle_single_job_route_with_all_spans() {
    // Create a route with only one job
    let test_cases = vec![
        // Distance is the whole route (0->10 + 10->0 = 20) for every span; only the paid
        // duration differs.
        (Some(RouteCostSpan::DepotToDepot), 20., 40.),   // duration 40-0=40
        (Some(RouteCostSpan::DepotToLastJob), 20., 20.), // duration 20-0=20
        (Some(RouteCostSpan::FirstJobToDepot), 20., 20.), // duration 40-20=20
        (Some(RouteCostSpan::FirstJobToLastJob), 20., 0.), // one job: first and last coincide
    ];

    for (span, expected_distance, expected_duration) in test_cases {
        let mut vehicle = TestVehicleBuilder::default().id("v1").details(vec![create_detail(0, 0)]).build();

        if let Some(s) = span {
            vehicle.dimens.set_route_cost_span(s);
        }

        let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

        let route = RouteBuilder::default()
            .with_vehicle(&fleet, "v1")
            .with_start({
                let mut start = ActivityBuilder::default().build();
                start.place.location = 0;
                start.schedule = Schedule::new(0., 0.);
                start.job = None;
                start
            })
            .with_end({
                let mut end = ActivityBuilder::default().build();
                end.place.location = 0;
                end.schedule = Schedule::new(40., 40.);
                end.job = None;
                end
            })
            .add_activities(vec![
                // Single job at location 10: arrive at 20 (with service), depart at 20
                create_activity_with_location_and_schedule(10, 20., 20.),
            ])
            .build();

        let mut route_ctx = RouteContextBuilder::default().with_route(route).build();
        let transport = TestTransportCost::default();

        update_statistics(&mut route_ctx, &transport);

        let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
        let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

        assert_eq!(
            total_distance, expected_distance,
            "Single job route with {:?} should have distance {}",
            span, expected_distance
        );
        assert_eq!(
            total_duration, expected_duration,
            "Single job route with {:?} should have duration {}",
            span, expected_duration
        );
    }
}

/// Creates an open VRP route (no end depot) with:
/// - Depot at location 0 (start only)
/// - Job 1 at location 10
/// - Job 2 at location 30
/// - Job 3 at location 60
///
/// With TestTransportCost (distance = |to - from|):
/// - Depot(0) -> Job1(10): distance = 10
/// - Job1(10) -> Job2(30): distance = 20
/// - Job2(30) -> Job3(60): distance = 30
///
/// Total distance for all spans (no return to depot):
/// - DepotToDepot: 10 + 20 + 30 = 60 (same as DepotToLastJob since no return)
/// - DepotToLastJob: 10 + 20 + 30 = 60
/// - FirstJobToDepot: 20 + 30 = 50 (same as FirstJobToLastJob since no return)
/// - FirstJobToLastJob: 20 + 30 = 50
fn create_open_vrp_route_with_cost_span(cost_span: Option<RouteCostSpan>) -> (RouteContext, TestTransportCost) {
    let mut vehicle = TestVehicleBuilder::default().id("v1").details(vec![create_open_detail(0)]).build();

    if let Some(span) = cost_span {
        vehicle.dimens.set_route_cost_span(span);
    }

    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    // Build route with start at 0, jobs at 10, 30, 60, NO end depot
    let route = RouteBuilder::default()
        .with_vehicle(&fleet, "v1")
        .with_start({
            let mut start = ActivityBuilder::default().build();
            start.place.location = 0;
            start.schedule = Schedule::new(0., 0.);
            start.job = None;
            start
        })
        // No end depot - open VRP
        .add_activities(vec![
            create_activity_with_location_and_schedule(10, 10., 10.),
            create_activity_with_location_and_schedule(30, 30., 30.),
            create_activity_with_location_and_schedule(60, 60., 60.),
        ])
        .build();

    let route_ctx = RouteContextBuilder::default().with_route(route).build();

    (route_ctx, TestTransportCost::default())
}

#[test]
fn can_calculate_statistics_for_open_vrp_with_depot_to_depot_span() {
    let (mut route_ctx, transport) = create_open_vrp_route_with_cost_span(Some(RouteCostSpan::DepotToDepot));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Open VRP: no return to depot, so distance is depot to last job
    // Distance: 0->10 + 10->30 + 30->60 = 10 + 20 + 30 = 60
    assert_eq!(total_distance, 60., "Open VRP DepotToDepot distance should be 60");
    // Duration: last_job.departure(60) - start.departure(0) = 60
    assert_eq!(total_duration, 60., "Open VRP DepotToDepot duration should be 60");
}

#[test]
fn can_calculate_statistics_for_open_vrp_with_depot_to_last_job_span() {
    let (mut route_ctx, transport) = create_open_vrp_route_with_cost_span(Some(RouteCostSpan::DepotToLastJob));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Distance: 0->10 + 10->30 + 30->60 = 10 + 20 + 30 = 60
    assert_eq!(total_distance, 60., "Open VRP DepotToLastJob distance should be 60");
    // Duration: last_job.departure(60) - start.departure(0) = 60
    assert_eq!(total_duration, 60., "Open VRP DepotToLastJob duration should be 60");
}

#[test]
fn can_calculate_statistics_for_open_vrp_with_first_job_to_depot_span() {
    let (mut route_ctx, transport) = create_open_vrp_route_with_cost_span(Some(RouteCostSpan::FirstJobToDepot));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Span-blind, and an open tour has no end depot: 0->10 + 10->30 + 30->60 = 60.
    assert_eq!(total_distance, 60., "Open VRP FirstJobToDepot distance is the whole route");
    // Duration: last_job.departure(60) - first_job.arrival(10) = 50
    assert_eq!(total_duration, 50., "Open VRP FirstJobToDepot duration should be 50");
}

#[test]
fn can_calculate_statistics_for_open_vrp_with_first_job_to_last_job_span() {
    let (mut route_ctx, transport) = create_open_vrp_route_with_cost_span(Some(RouteCostSpan::FirstJobToLastJob));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Span-blind, and an open tour has no end depot: 0->10 + 10->30 + 30->60 = 60.
    assert_eq!(total_distance, 60., "Open VRP FirstJobToLastJob distance is the whole route");
    // Duration: last_job.departure(60) - first_job.arrival(10) = 50
    assert_eq!(total_duration, 50., "Open VRP FirstJobToLastJob duration should be 50");
}

fn create_feasibility_detail(
    start_loc: Location,
    end_loc: Location,
    time_start: Timestamp,
    time_end: Timestamp,
) -> VehicleDetail {
    VehicleDetail {
        start: Some(VehiclePlace {
            location: start_loc,
            time: TimeInterval { earliest: Some(time_start), latest: None },
        }),
        end: Some(VehiclePlace { location: end_loc, time: TimeInterval { earliest: None, latest: Some(time_end) } }),
    }
}

fn create_feasibility_route(
    reserved_time: ReservedTimeSpan,
    activities: Vec<(Location, (Timestamp, Timestamp), f64)>,
) -> (Arc<dyn crate::models::problem::ActivityCost>, Arc<dyn crate::models::problem::TransportCost>, RouteContext) {
    let detail = create_feasibility_detail(0, 0, 0., 100.);
    let vehicle = TestVehicleBuilder::default().id("v1").details(vec![detail]).build();
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();
    let actor = fleet.actors.first().unwrap().clone();

    let reserved_times_idx =
        vec![(actor.clone(), vec![reserved_time])].into_iter().collect::<std::collections::HashMap<_, _>>();

    let activity_cost: Arc<dyn crate::models::problem::ActivityCost> =
        Arc::new(DynamicActivityCost::new(reserved_times_idx.clone()).unwrap());
    let transport: Arc<dyn crate::models::problem::TransportCost> =
        Arc::new(DynamicTransportCost::new(reserved_times_idx, Arc::new(TestTransportCost::default())).unwrap());

    let acts = activities.into_iter().map(|(loc, (start, end), dur)| {
        ActivityBuilder::with_location_tw_and_duration(loc, TimeWindow::new(start, end), dur).build()
    });

    let mut route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(acts).build())
        .build();

    update_route_schedule(&mut route_ctx, activity_cost.as_ref(), transport.as_ref());

    (activity_cost, transport, route_ctx)
}

#[test]
fn is_schedule_feasible_returns_true_for_feasible_route_with_reserved_time() {
    // Reserved time at t=25, duration=5. Activity at loc=10, tw=(0,100), dur=10.
    // Activity arrives at 10, departs at 20. Reserved time at 25 doesn't cause Break.
    let reserved_time = ReservedTimeSpan { time: TimeSpan::Window(TimeWindow::new(25., 25.)), duration: 5. };
    let (activity_cost, transport, route_ctx) = create_feasibility_route(reserved_time, vec![(10, (0., 100.), 10.)]);

    assert!(is_schedule_feasible(route_ctx.route(), activity_cost.as_ref(), transport.as_ref()));
}

#[test]
fn is_schedule_feasible_returns_false_when_break_exceeds_activity_tw() {
    // Reserved time at t=9, duration=12 means break runs from 9 to 21.
    // Activity at loc=10, tw=(0,20), dur=10. With plain transport, arrival=10.
    // estimate_departure: activity_start=10, departure=20, schedule=(10,20)
    // reserved time intersects, extra_duration=12, 10+12=22 > 20 → Break
    //
    // We use DynamicActivityCost (has reserved times) but plain TestTransportCost
    // (no reserved time in transport) so that arrival is 10, not shifted.
    let detail = create_feasibility_detail(0, 0, 0., 100.);
    let vehicle = TestVehicleBuilder::default().id("v1").details(vec![detail]).build();
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();
    let actor = fleet.actors.first().unwrap().clone();

    let reserved_time = ReservedTimeSpan { time: TimeSpan::Window(TimeWindow::new(9., 9.)), duration: 12. };
    let reserved_times_idx =
        vec![(actor.clone(), vec![reserved_time])].into_iter().collect::<std::collections::HashMap<_, _>>();

    let activity_cost: Arc<dyn crate::models::problem::ActivityCost> =
        Arc::new(DynamicActivityCost::new(reserved_times_idx).unwrap());
    let transport = TestTransportCost::default();

    let acts = vec![ActivityBuilder::with_location_tw_and_duration(10, TimeWindow::new(0., 20.), 10.).build()];
    let route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(acts).build())
        .build();

    assert!(!is_schedule_feasible(route_ctx.route(), activity_cost.as_ref(), &transport));
}

/// The anchor must skip an activity whose own window is derived from it. Anchoring on the break
/// defines its window in terms of itself, and it then sits before the first customer at whatever
/// time the search happened to give it.
#[test]
fn can_skip_an_offset_anchored_activity_when_choosing_the_anchor() {
    let mut vehicle = TestVehicleBuilder::default().id("v1").details(vec![create_detail(0, 0)]).build();
    vehicle.dimens.set_route_cost_span(RouteCostSpan::FirstJobToLastJob);
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(vehicle).build();

    // A required break placed by an offset span, landing ahead of every customer.
    let offset_activity = {
        let mut activity = create_activity_with_location_and_schedule(5, 20., 22.);
        let mut single = TestSingleBuilder::default().location(Some(5)).build();
        single.places.first_mut().unwrap().times = vec![TimeSpan::Offset(TimeOffset::new(7., 7.))];
        activity.job = Some(Arc::new(single));
        activity
    };

    let route = RouteBuilder::default()
        .with_vehicle(&fleet, "v1")
        .with_start({
            let mut start = ActivityBuilder::default().build();
            start.place.location = 0;
            start.schedule = Schedule::new(0., 0.);
            start.job = None;
            start
        })
        .add_activity(offset_activity)
        .add_activity(create_activity_with_location_and_schedule(10, 40., 41.))
        .build();

    // 40. is the customer's arrival, 20. the break's — the break must not become the anchor.
    assert_eq!(get_offset_anchor(&route), 40.);
}

/// ⚠️ These numbers are duplicated on the app side, on purpose. The territory objective balances
/// on what `calculate_route_duration` returns, and the app bills what its own `ResolvePaidSeconds`
/// returns; two implementations of one formula drift silently, so `tests/Unit/Shared/PayPeriodTest.php`
/// states the same route — depot departure 0, first job arrival 10, last job departure 60, depot
/// arrival 130 — and asserts the same four spans.
#[test]
fn total_distance_covers_depot_legs_while_duration_does_not() {
    // Miles are vehicle cost: they are incurred on the commute legs whether or not the technician
    // is on the clock for them, and the cost model bills the whole route. Duration is the
    // opposite and keeps the span the vehicle is paid for.
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::FirstJobToLastJob));

    update_statistics(&mut route_ctx, &transport);

    let total_distance = route_ctx.state().get_total_distance().copied().unwrap_or(0.);
    let total_duration = route_ctx.state().get_total_duration().copied().unwrap_or(0.);

    // Whole route: 0->10 + 10->30 + 30->60 + 60->0 = 120, both depot legs included.
    assert_eq!(total_distance, 120., "distance must cover the depot legs whatever the span");
    // Paid span only: last_job.departure(60) - first_job.arrival(10) = 50.
    assert_eq!(total_duration, 50., "duration must stay inside the paid span");
}

#[test]
fn paid_working_duration_is_the_span_without_its_idle() {
    // The fixture's activities carry no service time, so over depot-to-depot the worked duration
    // is the whole route's travel: 0->10 + 10->30 + 30->60 + 60->0 = 120. The span is 130, the
    // extra ten seconds being idle the schedule leaves between arrival and departure.
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::DepotToDepot));

    update_statistics(&mut route_ctx, &transport);

    assert_eq!(route_ctx.state().get_paid_working_duration().copied().unwrap_or(0.), 120.);
    assert_eq!(route_ctx.state().get_total_duration().copied().unwrap_or(0.), 130.);
}

#[test]
fn paid_working_duration_counts_only_the_legs_the_span_pays_for() {
    // first-job-to-last-job: 10->30 + 30->60 = 50, neither depot leg.
    let (mut route_ctx, transport) = create_test_route_with_cost_span(Some(RouteCostSpan::FirstJobToLastJob));

    update_statistics(&mut route_ctx, &transport);

    assert_eq!(route_ctx.state().get_paid_working_duration().copied().unwrap_or(0.), 50.);
    // ...while the distance beside it is span-blind and still covers the whole route.
    assert_eq!(route_ctx.state().get_total_distance().copied().unwrap_or(0.), 120.);
}
