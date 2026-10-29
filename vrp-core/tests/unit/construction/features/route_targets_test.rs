use super::*;
use crate::construction::heuristics::ActivityContext;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{TestSingleBuilder, TestTransportCost};
use crate::helpers::models::solution::test_actor;
use crate::models::common::{Schedule, TimeWindow};
use crate::models::solution::{Place, Tour};

custom_dimension!(TestJobValue typeof Float);

/// A stop of the fixture tour: location, arrival, service duration and value; `None` value marks a break.
type Stop = (Location, Float, Float, Option<Float>);

fn create_single(location: Location, duration: Float, value: Option<Float>) -> Arc<Single> {
    let mut builder = TestSingleBuilder::default();
    builder.location(Some(location)).duration(duration);
    match value {
        Some(value) => builder.id("job").dimens_mut().set_test_job_value(value),
        None => builder.id("break").dimens_mut(),
    };

    builder.build_shared()
}

fn create_activity(single: Option<Arc<Single>>, location: Location, arrival: Float, duration: Float) -> Activity {
    Activity {
        place: Place { idx: 0, location, duration, time: TimeWindow::max() },
        schedule: Schedule::new(arrival, arrival + duration),
        job: single,
        commute: None,
    }
}

/// A closed tour from and back to a depot at location 0, leaving at time 0 and back at `end_arrival`.
fn create_route(stops: &[Stop], end_arrival: Float) -> Route {
    let mut tour = Tour::default();
    tour.set_start(create_activity(None, 0, 0., 0.));
    tour.set_end(create_activity(None, 0, end_arrival, 0.));
    stops.iter().for_each(|&(location, arrival, duration, value)| {
        tour.insert_last(create_activity(Some(create_single(location, duration, value)), location, arrival, duration));
    });

    Route { actor: test_actor(), tour }
}

/// Depot 0 -> job at 10 (arrives 10, serves 5) -> break at 20 (25..55) -> job at 30 (arrives 65, serves 5) -> depot
/// (arrives 100). Distance equals driving time under the test transport.
///
/// | span                 | distance | elapsed |
/// |----------------------|----------|---------|
/// | first-job-to-last-job| 20       | 60      |
/// | depot-to-depot       | 60       | 100     |
/// | depot-to-last-job    | 30       | 70      |
/// | first-job-to-depot   | 50       | 90      |
///
/// The break inside every span is 30, service time is 10, value 3 + 5 and there are two jobs.
fn create_fixture_route() -> Route {
    create_route(&[(10, 10., 5., Some(3.)), (20, 25., 30., None), (30, 65., 5., Some(5.))], 100.)
}

fn create_shared(targets: Vec<RouteTarget>, aggregate: RouteTargetAggregate) -> RouteTargetsShared {
    RouteTargetsShared {
        transport: TestTransportCost::new_shared(),
        targets,
        aggregate,
        job_value_fn: Arc::new(|job| job.dimens().get_test_job_value().copied().unwrap_or(0.)),
        is_break_fn: Arc::new(|single| single.dimens.get_job_id().is_some_and(|id| id == "break")),
    }
}

/// The identity curve: a route scores exactly its reading.
fn identity() -> Vec<(Float, Float)> {
    vec![(0., 0.), (1., 1.)]
}

fn target(metric: RouteMetric, span: RouteCostSpan, include_breaks: bool, per: RouteTargetPer) -> RouteTarget {
    RouteTarget { metric, span, include_breaks, per, points: identity() }
}

fn read(target: RouteTarget, route: &Route) -> Float {
    let shared = create_shared(vec![target], RouteTargetAggregate::Sum);
    shared.score(&shared.measure(route))
}

parameterized_test! {can_read_metric_over_span, (metric, span, include_breaks, per, expected), {
    can_read_metric_over_span_impl(metric, span, include_breaks, per, expected);
}}

can_read_metric_over_span! {
    case01_distance_first_job_to_last_job: (RouteMetric::Distance, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 20.),
    case02_distance_depot_to_depot: (RouteMetric::Distance, RouteCostSpan::DepotToDepot, false, RouteTargetPer::Route, 60.),
    case03_distance_depot_to_last_job: (RouteMetric::Distance, RouteCostSpan::DepotToLastJob, false, RouteTargetPer::Route, 30.),
    case04_distance_first_job_to_depot: (RouteMetric::Distance, RouteCostSpan::FirstJobToDepot, false, RouteTargetPer::Route, 50.),
    case05_driving_time_first_job_to_last_job: (RouteMetric::DrivingTime, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 20.),
    case06_driving_time_depot_to_depot: (RouteMetric::DrivingTime, RouteCostSpan::DepotToDepot, false, RouteTargetPer::Route, 60.),
    case07_duration_breaks_excluded: (RouteMetric::Duration, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 30.),
    case08_duration_breaks_included: (RouteMetric::Duration, RouteCostSpan::FirstJobToLastJob, true, RouteTargetPer::Route, 60.),
    case09_duration_depot_to_depot_breaks_excluded: (RouteMetric::Duration, RouteCostSpan::DepotToDepot, false, RouteTargetPer::Route, 70.),
    case10_duration_depot_to_depot_breaks_included: (RouteMetric::Duration, RouteCostSpan::DepotToDepot, true, RouteTargetPer::Route, 100.),
    case11_service_time: (RouteMetric::ServiceTime, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 10.),
    case12_value: (RouteMetric::Value, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 8.),
    case13_activities_without_breaks: (RouteMetric::Activities, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Route, 2.),
    case14_distance_per_activity: (RouteMetric::Distance, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Activity, 10.),
    case15_value_per_activity: (RouteMetric::Value, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Activity, 4.),
    case16_service_time_per_hour: (RouteMetric::ServiceTime, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Hour, 1200.),
    case17_value_per_hour: (RouteMetric::Value, RouteCostSpan::FirstJobToLastJob, false, RouteTargetPer::Hour, 960.),
}

fn can_read_metric_over_span_impl(
    metric: RouteMetric,
    span: RouteCostSpan,
    include_breaks: bool,
    per: RouteTargetPer,
    expected: Float,
) {
    let route = create_fixture_route();

    let actual = read(target(metric, span, include_breaks, per), &route);

    assert!((actual - expected).abs() < 1e-9, "expected {expected}, got {actual}");
}

parameterized_test! {can_read_curve, (points, x, expected), {
    can_read_curve_impl(points, x, expected);
}}

can_read_curve! {
    case01_on_a_point: (vec![(10., 0.), (20., 1.), (30., 3.)], 20., 1.),
    case02_between_points: (vec![(10., 0.), (20., 1.), (30., 3.)], 25., 2.),
    case03_beyond_last_point_keeps_last_slope: (vec![(10., 0.), (20., 1.), (30., 3.)], 40., 5.),
    case04_before_first_point_keeps_first_slope: (vec![(10., 2.), (20., 1.), (30., 0.)], 0., 3.),
    case05_never_below_zero_before_first: (vec![(10., 0.), (20., 1.)], 0., 0.),
    case06_never_below_zero_beyond_last: (vec![(10., 2.), (20., 0.)], 40., 0.),
}

fn can_read_curve_impl(points: Vec<(Float, Float)>, x: Float, expected: Float) {
    let actual = curve(&points, x);

    assert!((actual - expected).abs() < 1e-9, "expected {expected}, got {actual}");
}

parameterized_test! {can_aggregate_targets, (aggregate, expected), {
    can_aggregate_targets_impl(aggregate, expected);
}}

can_aggregate_targets! {
    case01_sum: (RouteTargetAggregate::Sum, 22.),
    case02_sum_of_squares: (RouteTargetAggregate::SumOfSquares, 404.),
    case03_max: (RouteTargetAggregate::Max, 20.),
}

fn can_aggregate_targets_impl(aggregate: RouteTargetAggregate, expected: Float) {
    let route = create_fixture_route();
    let shared = create_shared(
        vec![
            RouteTarget::new(RouteMetric::Distance, identity()),
            RouteTarget::new(RouteMetric::Activities, identity()),
        ],
        aggregate,
    );

    let actual = shared.score(&shared.measure(&route));

    assert!((actual - expected).abs() < 1e-9, "expected {expected}, got {actual}");
}

#[test]
fn can_score_empty_route_as_zero() {
    let route = create_route(&[], 0.);
    let shared = create_shared(
        vec![RouteTarget::new(RouteMetric::Value, vec![(0., 5.), (10., 0.)])],
        RouteTargetAggregate::SumOfSquares,
    );

    assert_eq!(shared.score(&shared.measure(&route)), 0.);
}

#[test]
fn can_score_route_with_only_breaks_as_zero() {
    let route = create_route(&[(20, 20., 30., None)], 70.);
    let shared = create_shared(
        vec![RouteTarget::new(RouteMetric::Distance, vec![(0., 5.), (10., 6.)])],
        RouteTargetAggregate::Sum,
    );

    assert_eq!(shared.score(&shared.measure(&route)), 0.);
}

#[test]
fn can_score_per_hour_target_as_zero_when_working_time_is_zero() {
    let route = create_route(&[(10, 10., 0., Some(3.))], 20.);
    let target =
        RouteTarget { per: RouteTargetPer::Hour, ..RouteTarget::new(RouteMetric::Value, vec![(0., 5.), (10., 0.)]) };

    assert_eq!(read(target, &route), 0.);
}

#[test]
fn can_refuse_invalid_curves() {
    let too_short = RouteTarget::new(RouteMetric::Distance, vec![(0., 0.)]);
    let descending = RouteTarget::new(RouteMetric::Distance, vec![(10., 0.), (5., 1.)]);
    let repeated = RouteTarget::new(RouteMetric::Distance, vec![(10., 0.), (10., 1.)]);
    let valid = RouteTarget::new(RouteMetric::Distance, vec![(10., 0.), (20., 1.)]);

    assert!(too_short.validate().is_err());
    assert!(descending.validate().is_err());
    assert!(repeated.validate().is_err());
    assert!(valid.validate().is_ok());
    assert!(
        RouteTargetsBuilder::new("route_targets")
            .set_transport(TestTransportCost::new_shared())
            .set_targets(vec![descending])
            .build()
            .is_err()
    );
}

parameterized_test! {can_estimate_insertion_as_measured_delta, (span, expected), {
    can_estimate_insertion_as_measured_delta_impl(span, expected);
}}

can_estimate_insertion_as_measured_delta! {
    case01_first_job_to_last_job: (RouteCostSpan::FirstJobToLastJob, 10.),
    case02_depot_to_depot: (RouteCostSpan::DepotToDepot, 20.),
    case03_depot_to_last_job: (RouteCostSpan::DepotToLastJob, 10.),
    case04_first_job_to_depot: (RouteCostSpan::FirstJobToDepot, 20.),
}

/// Inserting a job at 20 after the only job at 10 of a route costs, per span, the distance it adds.
fn can_estimate_insertion_as_measured_delta_impl(span: RouteCostSpan, expected: Float) {
    let targets = vec![RouteTarget { span, ..RouteTarget::new(RouteMetric::Distance, identity()) }];
    let feature = RouteTargetsBuilder::new("route_targets")
        .set_transport(TestTransportCost::new_shared())
        .set_targets(targets)
        .set_aggregate(RouteTargetAggregate::Sum)
        .build()
        .unwrap();
    let objective = feature.objective.clone().unwrap();
    let mut route_ctx = RouteContext::new(test_actor());
    *route_ctx.route_mut() = create_route(&[(10, 10., 5., Some(1.))], 25.);
    feature.state.as_ref().unwrap().accept_route_state(&mut route_ctx);
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;
    let single = create_single(20, 5., Some(1.));
    let job = Job::Single(single.clone());
    let tour = &route_ctx.route().tour;
    let target = create_activity(Some(single), 20, 25., 5.);
    let activity_ctx = ActivityContext { index: 2, prev: tour.get(1).unwrap(), target: &target, next: tour.get(2) };

    let actual = objective.estimate(&MoveContext::route(&solution_ctx, &route_ctx, &job))
        + objective.estimate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx));

    assert!((actual - expected).abs() < 1e-9, "expected {expected}, got {actual}");
}

#[test]
fn can_cache_fitness_over_routes() {
    let feature = RouteTargetsBuilder::new("route_targets")
        .set_transport(TestTransportCost::new_shared())
        .set_targets(vec![RouteTarget::new(RouteMetric::Distance, identity())])
        .set_aggregate(RouteTargetAggregate::Sum)
        .build()
        .unwrap();
    let mut filled = RouteContext::new(test_actor());
    *filled.route_mut() = create_fixture_route();
    let mut empty = RouteContext::new(test_actor());
    *empty.route_mut() = create_route(&[], 0.);
    let mut insertion_ctx = TestInsertionContextBuilder::default().with_routes(vec![filled, empty]).build();

    feature.state.as_ref().unwrap().accept_solution_state(&mut insertion_ctx.solution);

    assert_eq!(insertion_ctx.solution.state.get_route_targets_fitness().copied(), Some(20.));
    assert_eq!(feature.objective.as_ref().unwrap().fitness(&insertion_ctx), 20.);
}
