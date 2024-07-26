use super::*;
use crate::construction::enablers::LatestArrivalActivityState;
use crate::construction::features::TransportFeatureBuilder;
use crate::construction::heuristics::*;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::problem::*;
use crate::models::{Feature, ViolationCode};

const VIOLATION_CODE: ViolationCode = ViolationCode(1);

type VehicleData = (Location, Location, Timestamp, Timestamp);
type ActivityData = (Location, (Timestamp, Timestamp), Duration);

fn create_detail(
    locations: (Option<Location>, Option<Location>),
    time: Option<(Timestamp, Timestamp)>,
) -> VehicleDetail {
    let (start_location, end_location) = locations;
    VehicleDetail {
        start: start_location.map(|location| VehiclePlace {
            location,
            time: time.map_or(Default::default(), |(start, _)| TimeInterval { earliest: Some(start), latest: None }),
        }),
        end: end_location.map(|location| VehiclePlace {
            location,
            time: time.map_or(Default::default(), |(_, end)| TimeInterval { earliest: None, latest: Some(end) }),
        }),
    }
}

parameterized_test! {can_search_for_reserved_time, (times, tests), {
    can_search_for_reserved_time_impl(times, tests);
}}

can_search_for_reserved_time! {
    case01: (vec![((5., 5.), 5.), ((20., 20.), 10.)],
        vec![((6., 6.), Some(0)), ((2., 6.), Some(0)), ((10., 11.), None), ((2., 4.), None),
             ((10., 21.), Some(1)), ((25., 27.), Some(1)), ((29., 31.), Some(1)),
             ((0., 3.), None), ((31., 33.), None)]),
    case02: (vec![((0.,0.), 10.), ((5., 5.), 10.)], vec![]),
}

fn can_search_for_reserved_time_impl(
    times: Vec<((Timestamp, Timestamp), Duration)>,
    tests: Vec<((Timestamp, Timestamp), Option<usize>)>,
) {
    let route_ctx = RouteContextBuilder::default().build();
    let reserved_times = vec![(
        route_ctx.route().actor.clone(),
        times
            .iter()
            .cloned()
            .map(|((start, end), duration)| ReservedTimeSpan {
                time: TimeSpan::Window(TimeWindow::new(start, end)),
                duration,
            })
            .collect::<Vec<_>>(),
    )]
    .into_iter()
    .collect();

    let reserved_time_fn = create_reserved_times_fn(reserved_times);

    if let Ok(reserved_time_fn) = reserved_time_fn {
        tests.iter().enumerate().for_each(|(test_idx, ((s, e), expected))| {
            let interval = TimeWindow::new(*s, *e);
            let expected = expected.and_then(|idx| times.get(idx)).map(|((s, e), _)| TimeWindow::new(*s, *e));

            let result = (reserved_time_fn)(route_ctx.route(), &interval);

            assert_eq!(result.map(|r| r.time), expected, "test {test_idx} is failed");
        });
    } else {
        assert!(tests.is_empty())
    }
}

fn create_feature_and_route(
    vehicle_detail_data: VehicleData,
    activities: Vec<ActivityData>,
    reserved_time: ReservedTimeSpan,
) -> (ReservedTimesFn, Feature, RouteContext) {
    let (location_start, location_end, time_start, time_end) = vehicle_detail_data;

    let activities = activities.into_iter().map(|(loc, (start, end), dur)| {
        ActivityBuilder::with_location_tw_and_duration(loc, TimeWindow::new(start, end), dur).build()
    });
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![TestVehicleBuilder::default()
            .id("v1")
            .details(vec![create_detail((Some(location_start), Some(location_end)), Some((time_start, time_end)))])
            .build()])
        .build();
    let reserved_times_idx =
        vec![(fleet.actors.first().unwrap().clone(), vec![reserved_time])].into_iter().collect::<HashMap<_, _>>();
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").add_activities(activities).build())
        .build();
    let feature = TransportFeatureBuilder::new("minimize_costs")
        .set_violation_code(VIOLATION_CODE)
        .set_transport_cost(Arc::new(
            DynamicTransportCost::new(reserved_times_idx.clone(), Arc::new(TestTransportCost::default())).unwrap(),
        ))
        .set_activity_cost(Arc::new(DynamicActivityCost::new(reserved_times_idx.clone()).unwrap()))
        .build_minimize_cost()
        .unwrap();
    feature.state.as_ref().unwrap().accept_route_state(&mut route_ctx);

    (create_reserved_times_fn(reserved_times_idx).unwrap(), feature, route_ctx)
}

fn get_schedules(route_ctx: &RouteContext) -> Vec<(Timestamp, Timestamp)> {
    route_ctx.route().tour.all_activities().map(|a| (a.schedule.arrival, a.schedule.departure)).collect()
}

parameterized_test! {can_update_state_for_reserved_time, (vehicle_detail_data, reserved_time, activities, late_arrival_expected, expected_schedules), {
    let reserved_time = ReservedTimeSpan {
        time: TimeSpan::Window(TimeWindow::new(reserved_time.0, reserved_time.0)),
        duration: reserved_time.1 - reserved_time.0,
    };
    can_update_state_for_reserved_time_impl(vehicle_detail_data, reserved_time, activities, late_arrival_expected, expected_schedules);
}}

can_update_state_for_reserved_time! {
    case01_single_outside: ((0, 0, 0., 100.), (25., 30.),
              vec![(10, (0., 100.), 10.)],
              vec![Some(0.), Some(80.), None],
              vec![(0., 0.), (10., 20.), (35., 35.)]),

    case02_single_inside: ((0, 0, 0., 100.), (25., 30.),
              vec![(20, (0., 25.), 10.)],
              vec![Some(0.), Some(20.), None],
              vec![(0., 0.), (20., 35.), (55., 55.)]),

    case03_two_inside_travel: ((0, 0, 0., 100.), (25., 30.),
              vec![(10, (0., 20.), 10.), (20, (0., 40.), 10.)],
              vec![Some(0.), Some(15.), Some(40.), None],
              vec![(0., 0.), (10., 20.), (35., 45.), (65., 65.)]),

    case04_two_inside_service: ((0, 0, 0., 100.), (35., 40.),
              vec![(10, (0., 20.), 10.), (20, (0., 50.), 10.)],
              vec![Some(0.), Some(15.), Some(50.), None],
              vec![(0., 0.), (10., 20.), (30., 45.), (65., 65.)]),
}

fn can_update_state_for_reserved_time_impl(
    vehicle_detail_data: VehicleData,
    reserved_time: ReservedTimeSpan,
    activities: Vec<ActivityData>,
    late_arrival_expected: Vec<Option<f64>>,
    expected_schedules: Vec<(Timestamp, Timestamp)>,
) {
    let (_, _, route_ctx) = create_feature_and_route(vehicle_detail_data, activities, reserved_time);

    let schedules = get_schedules(&route_ctx);
    let late_arrival_result = (0..route_ctx.route().tour.total())
        .map(|activity_idx| route_ctx.state().get_latest_arrival_at(activity_idx).copied())
        .collect::<Vec<_>>();

    assert_eq!(schedules, expected_schedules);
    assert_eq!(late_arrival_result, late_arrival_expected);
}

parameterized_test! {can_evaluate_activity, (vehicle_detail_data, reserved_time, target, activities, expected_schedules), {
    let reserved_time = ReservedTimeSpan {
        time: TimeSpan::Window(TimeWindow::new(reserved_time.0, reserved_time.0)),
        duration: reserved_time.1 - reserved_time.0,
    };
    can_evaluate_activity_impl(vehicle_detail_data, reserved_time, target, activities, expected_schedules);
}}

can_evaluate_activity! {
    case01_break_starts_at_target_then_next_at_end:
        ((0, 0, 0., 100.), (10., 20.),
        (10, (0., 100.), 10.),
        vec![(20, (0., 40.), 10.)],
        vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

    case02_break_starts_at_target_then_next_is_late:
        ((0, 0, 0., 100.), (10., 20.),
        (10, (0., 100.), 10.),
        vec![(20, (0., 39.), 10.)],
        vec![]),

    case03_break_with_waiting_time:
        ((0, 0, 0., 100.), (10., 20.),
        (10, (20., 100.), 10.),
        vec![(20, (0., 100.), 10.)],
        vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

    case04_break_during_traveling:
        ((0, 0, 0., 100.), (5., 10.),
        (10, (0., 100.), 10.),
        vec![(20, (0., 100.), 10.)],
        vec![(0., 0.), (15., 25.), (35., 45.), (65., 65.)]),

    case05_break_on_whole_time_window_exclusive:
        ((0, 0, 0., 100.), (10., 20.),
        (10, (0., 20.), 10.),
        vec![(20, (0., 100.), 10.)],
        vec![(0., 0.), (10., 30.), (40., 50.), (70., 70.)]),

    case06_break_on_whole_time_window_inclusive:
        ((0, 0, 0., 100.), (10., 21.),
        (10, (0., 20.), 10.),
        vec![(20, (0., 100.), 10.)],
        vec![]),

    case07_break_on_whole_time_window_inclusive:
        ((0, 0, 0., 100.), (9., 21.),
        (10, (10., 20.), 10.),
        vec![(20, (0., 100.), 10.)],
        vec![]),

    case08_break_constraints_next:
        ((0, 0, 0., 100.), (50., 60.),
        (30, (0., 100.), 10.),
        vec![(20, (0., 50.), 10.)],
        vec![]),

    case09_break_constraints_next:
        ((0, 0, 0., 100.), (45., 60.),
        (30, (0., 100.), 10.),
        vec![(20, (0., 50.), 10.)],
        vec![]),
}

fn can_evaluate_activity_impl(
    vehicle_detail_data: VehicleData,
    reserved_time: ReservedTimeSpan,
    target: ActivityData,
    activities: Vec<ActivityData>,
    expected_schedules: Vec<(Timestamp, Timestamp)>,
) {
    let (_, feature, mut route_ctx) = create_feature_and_route(vehicle_detail_data, activities, reserved_time);
    let (feature_constraint, feature_state) = (feature.constraint.unwrap(), feature.state.unwrap());
    let (loc, (start, end), dur) = target;
    let prev = route_ctx.route().tour.get(0).unwrap();
    let target = ActivityBuilder::with_location_tw_and_duration(loc, TimeWindow::new(start, end), dur).build();
    let next = route_ctx.route().tour.get(1);
    let activity_ctx = ActivityContext { index: 0, prev, target: &target, next };

    let is_violation = feature_constraint.evaluate(&MoveContext::activity(&route_ctx, &activity_ctx)).is_some();

    assert_eq!(is_violation, expected_schedules.is_empty());
    if !is_violation {
        route_ctx.route_mut().tour.insert_at(target, 1);
        feature_state.accept_route_state(&mut route_ctx);
        assert_eq!(get_schedules(&route_ctx), expected_schedules)
    }
}

parameterized_test! {can_avoid_reserved_time_when_driving, (vehicle_detail_data, reserved_time, activities, expected_schedules), {
    can_avoid_reserved_time_when_driving_impl(vehicle_detail_data, reserved_time, activities, expected_schedules);
}}

can_avoid_reserved_time_when_driving! {
    case01_should_move_duration_to_serving: (
        (0, 0, 0., 100.), (10., 40., 5.),
        vec![(10, (0., 100.), 10.), (50, (0., 100.), 10.)],
        vec![(0., 0.), (10., 25.), (65., 75.), (125., 125.)]
    ),
    case02_should_keep_duration_at_driving: (
        (0, 0, 0., 100.), (30., 40., 5.),
        vec![(10, (0., 100.), 10.), (50, (0., 100.), 10.)],
        vec![(0., 0.), (10., 20.), (65., 75.), (125., 125.)]
    ),
}

fn can_avoid_reserved_time_when_driving_impl(
    vehicle_detail_data: VehicleData,
    reserved_time: (Timestamp, Timestamp, Duration),
    activities: Vec<ActivityData>,
    expected_schedules: Vec<(Timestamp, Timestamp)>,
) {
    let reserved_time = ReservedTimeSpan {
        time: TimeSpan::Offset(TimeOffset::new(reserved_time.0, reserved_time.1)),
        duration: reserved_time.2,
    };
    let (reserved_times_fn, _, mut route_ctx) =
        create_feature_and_route(vehicle_detail_data, activities, reserved_time);

    avoid_reserved_time_when_driving(route_ctx.route_mut(), &reserved_times_fn);

    assert_eq!(get_schedules(&route_ctx), expected_schedules)
}
