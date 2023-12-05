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
    case01: (None, true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(5.)),
    case02: (Some(3.), true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(3.)),
    case03: (Some(7.), true, vec![(0., 100.), (25., 100.), (0., 100.)], Some(5.)),
    case04: (None, true, vec![(0., 100.), (10., 100.), (42., 100.)], Some(12.)),

    case05: (None, false, vec![(12., 100.), (0., 100.), (0., 100.)], Some(2.)),
    case06: (None, false, vec![(10., 100.), (0., 100.), (0., 100.)], None),
    case07: (None, false, vec![(0., 100.), (25., 100.), (0., 100.)], None),
}

fn can_advance_departure_time_impl(
    latest: Option<f64>,
    optimize_whole_tour: bool,
    tws: Vec<TimeWindow>,
    expected: Option<f64>,
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
                    .add_activity(test_activity_with_location_and_tw(10, tw1.clone()))
                    .add_activity(test_activity_with_location_and_tw(20, tw2.clone()))
                    .add_activity(test_activity_with_location_and_tw(30, tw3.clone()))
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
    earliest: Option<f64>,
    start_departure: f64,
    latest_first_arrival: f64,
    tw: TimeWindow,
    total_duration_limit: Option<(f64, f64)>,
    expected: Option<f64>,
) {
    let state_keys = ScheduleStateKeys::default();
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
                .add_activity(test_activity_with_location_and_tw(10, tw))
                .build(),
        )
        .build();
    let (route, state) = route_ctx.as_mut();
    route.tour.get_mut(0).unwrap().schedule.departure = start_departure;
    let first = route.tour.get(1).unwrap();
    state.put_activity_state::<f64>(state_keys.latest_arrival, first, latest_first_arrival);

    if let Some((total, limit)) = total_duration_limit {
        state.put_route_state::<f64>(state_keys.total_duration, total);
        state.put_route_state(LIMIT_DURATION_KEY, limit);
    }

    let departure_time = try_recede_departure_time(&route_ctx, &state_keys, LIMIT_DURATION_KEY);

    assert_eq!(departure_time, expected);
}
