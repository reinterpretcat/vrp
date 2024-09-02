use super::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::*;
use crate::models::problem::*;

parameterized_test! {can_advance_departure_time, (latest, optimize_whole_tour, tws, expected), {
    let tws = tws.into_iter().map(|(start, end)| TimeWindow::new(start, end)).collect::<Vec<_>>();
    can_advance_departure_time_impl(latest, optimize_whole_tour, tws, expected);
}}

can_advance_departure_time! {
    case01: (None, true, vec![(0, 100), (25, 100), (0, 100)], Some(5)),
    case02: (Some(3), true, vec![(0, 100), (25, 100), (0, 100)], Some(3)),
    case03: (Some(7), true, vec![(0, 100), (25, 100), (0, 100)], Some(5)),
    case04: (None, true, vec![(0, 100), (10, 100), (42, 100)], Some(12)),

    case05: (None, false, vec![(12, 100), (0, 100), (0, 100)], Some(2)),
    case06: (None, false, vec![(10, 100), (0, 100), (0, 100)], None),
    case07: (None, false, vec![(0, 100), (25, 100), (0, 100)], None),
}

fn can_advance_departure_time_impl(
    latest: Option<Timestamp>,
    optimize_whole_tour: bool,
    tws: Vec<TimeWindow>,
    expected: Option<Timestamp>,
) {
    if let [tw1, tw2, tw3] = tws.as_slice() {
        let fleet = FleetBuilder::default()
            .add_driver(test_driver())
            .add_vehicle(Vehicle {
                details: vec![VehicleDetail {
                    start: Some(VehiclePlace { location: 0, time: TimeInterval { earliest: Some(0), latest } }),
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

        let departure_time = try_advance_departure_time(&route_ctx, &TestTransportCost::default(), optimize_whole_tour);

        assert_eq!(departure_time, expected);
    } else {
        unreachable!()
    }
}

parameterized_test! {can_recede_departure_time, (earliest, start_departure, latest_first_arrival, tw, duration_limit, expected), {
    can_recede_departure_time_impl(earliest, start_departure, latest_first_arrival, TimeWindow::new(tw.0, tw.1), duration_limit, expected);
}}

can_recede_departure_time! {
    case01: (Some(0), 0, 10, (10, 20), None, None),
    case02: (Some(0), 5, 10, (10, 20), None, None),
    case03: (Some(5), 10, 15, (10, 20), None, Some(5)),
    case04: (Some(5), 10, 20, (10, 20), None, Some(5)),
    case05: (None, 10, 50, (10, 20), None, Some(0)),
    case06: (Some(5), 10, 11, (10, 20), None, Some(9)),

    case07: (Some(0), 10, 20, (10, 20), Some((20, 30)), Some(0)),
    case08: (Some(0), 10, 20, (10, 20), Some((20, 25)), Some(5)),
    case09: (Some(0), 10, 20, (10, 20), Some((20, 20)), None),
}

fn can_recede_departure_time_impl(
    earliest: Option<Timestamp>,
    start_departure: Timestamp,
    latest_first_arrival: Timestamp,
    tw: TimeWindow,
    total_duration_limit: Option<(Duration, Duration)>,
    expected: Option<Timestamp>,
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
    state.set_latest_arrival_states(vec![Timestamp::default(), latest_first_arrival]);

    if let Some((total, limit)) = total_duration_limit {
        state.set_total_duration(total);
        state.set_limit_duration(limit);
    }

    let departure_time = try_recede_departure_time(&route_ctx);

    assert_eq!(departure_time, expected);
}
