use super::*;
use crate::construction::heuristics::{ActivityContext, RouteState};
use crate::helpers::construction::features::*;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Demand, SingleDimLoad};
use crate::models::problem::{Job, Vehicle};
use crate::models::solution::Activity;

const VIOLATION_CODE: ViolationCode = ViolationCode(2);

fn create_feature() -> Feature {
    CapacityFeatureBuilder::<SingleDimLoad>::new("capacity").set_violation_code(VIOLATION_CODE).build().unwrap()
}

fn create_test_vehicle(capacity: i32) -> Vehicle {
    TestVehicleBuilder::default().id("v1").capacity(capacity).build()
}

fn create_constraint_violation(stopped: bool) -> Option<ConstraintViolation> {
    Some(ConstraintViolation { code: VIOLATION_CODE, stopped, degree: None })
}

fn create_activity_with_simple_demand(size: i32) -> Activity {
    let job = TestSingleBuilder::default().demand(create_simple_demand(size)).build_shared();
    ActivityBuilder::default().job(Some(job)).build()
}

fn get_current_capacity_state(state: &RouteState, activity_idx: usize) -> i32 {
    state.get_current_capacity_at::<SingleDimLoad>(activity_idx).expect("expect single capacity").value
}

parameterized_test! {can_calculate_current_capacity_state_values, (s1, s2, s3, start, end, exp_s1, exp_s2, exp_s3), {
    can_calculate_current_capacity_state_values_impl(s1, s2, s3, start, end, exp_s1, exp_s2, exp_s3);
}}

can_calculate_current_capacity_state_values! {
    case01: (-1, 2, -3, 4, 2, 3, 5, 2),
    case02: (1, -2, 3, 2, 4, 3, 1, 4),
    case03: (0, 1, 0, 0, 1, 0, 1, 1),
}

#[allow(clippy::too_many_arguments)]
fn can_calculate_current_capacity_state_values_impl(
    s1: i32,
    s2: i32,
    s3: i32,
    start: i32,
    end: i32,
    exp_s1: i32,
    exp_s2: i32,
    exp_s3: i32,
) {
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(create_test_vehicle(10)).build();
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activity(create_activity_with_simple_demand(s1))
                .add_activity(create_activity_with_simple_demand(s2))
                .add_activity(create_activity_with_simple_demand(s3))
                .build(),
        )
        .build();
    create_feature().state.unwrap().accept_route_state(&mut route_ctx);

    let tour = &route_ctx.route().tour;
    let state = route_ctx.state();
    assert_eq!(get_current_capacity_state(state, 0), start);
    assert_eq!(get_current_capacity_state(state, tour.end_idx().unwrap()), end);
    assert_eq!(get_current_capacity_state(state, 1), exp_s1);
    assert_eq!(get_current_capacity_state(state, 2), exp_s2);
    assert_eq!(get_current_capacity_state(state, 3), exp_s3);
}

parameterized_test! {can_evaluate_demand_on_route, (size, expected), {
    can_evaluate_demand_on_route_impl(size, expected);
}}

can_evaluate_demand_on_route! {
    case01: (11, ConstraintViolation::fail(VIOLATION_CODE)),
    case02: (10, None),
    case03: (9, None),
}

fn can_evaluate_demand_on_route_impl(size: i32, expected: Option<ConstraintViolation>) {
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(create_test_vehicle(10)).build();
    let insertion_ctx = TestInsertionContextBuilder::default().build();
    let route_ctx =
        RouteContextBuilder::default().with_route(RouteBuilder::default().with_vehicle(&fleet, "v1").build()).build();
    let job = TestSingleBuilder::default().demand(create_simple_demand(size)).build_as_job_ref();

    let result =
        create_feature().constraint.unwrap().evaluate(&MoveContext::route(&insertion_ctx.solution, &route_ctx, &job));

    assert_eq!(result, expected);
}

parameterized_test! {can_evaluate_demand_on_activity, (sizes, neighbours, size, expected), {
    can_evaluate_demand_on_activity_impl(sizes, neighbours, size, expected);
}}

can_evaluate_demand_on_activity! {
    case01: (vec![1, 1], (1, 2), 1, None),
    case02: (vec![1, 1], (1, 2), 10, create_constraint_violation(false)),
    case03: (vec![-5, -5], (1, 2), -1, create_constraint_violation(true)),
    case04: (vec![5, 5], (1, 2), 1, create_constraint_violation(false)),
    case05: (vec![-5, 5], (1, 2), 1, None),
    case06: (vec![5, -5], (1, 2), 1, create_constraint_violation(false)),
    case07: (vec![4, -5], (1, 2),-1, None),
    case08: (vec![-3, -5, -2], (0, 1), -1, create_constraint_violation(true)),
    case09: (vec![-3, -5, -2], (0, 2), -1, create_constraint_violation(true)),
    case10: (vec![-3, -5, -2], (1, 3), -1, create_constraint_violation(true)),
    case11: (vec![-3, -5, -2], (3, 4), -1, create_constraint_violation(true)),
}

fn can_evaluate_demand_on_activity_impl(
    sizes: Vec<i32>,
    neighbours: (usize, usize),
    size: i32,
    expected: Option<ConstraintViolation>,
) {
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(create_test_vehicle(10)).build();
    let mut route_ctx = RouteContextBuilder::default()
        .with_route(
            RouteBuilder::default()
                .with_vehicle(&fleet, "v1")
                .add_activities(sizes.into_iter().map(create_activity_with_simple_demand))
                .build(),
        )
        .build();
    let feature = create_feature();
    feature.state.unwrap().accept_route_state(&mut route_ctx);
    let activity_ctx = ActivityContext {
        index: 0,
        prev: route_ctx.route().tour.get(neighbours.0).unwrap(),
        target: &create_activity_with_simple_demand(size),
        next: route_ctx.route().tour.get(neighbours.1),
    };
    let solution_ctx = TestInsertionContextBuilder::default().build().solution;

    let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(&solution_ctx, &route_ctx, &activity_ctx));

    assert_eq!(result, expected);
}

parameterized_test! {can_merge_jobs_with_demand, (cluster, candidate, expected), {
    can_merge_jobs_with_demand_impl(cluster, candidate, expected);
}}

can_merge_jobs_with_demand! {
    case01: (Some((1, 0, 0, 0)), Some((1, 0, 0, 0)), Ok((2, 0, 0, 0))),
    case02: (Some((1, 0, 1, 0)), Some((1, 0, 0, 0)), Ok((2, 0, 1, 0))),
    case03: (Some((0, 0, 1, 0)), Some((1, 0, 0, 0)), Ok((1, 0, 1, 0))),
    case04: (None, Some((1, 0, 0, 0)), Ok((1, 0, 0, 0))),
    case05: (Some((1, 0, 0, 0)), None, Ok((1, 0, 0, 0))),
    case06: (None, None, Err(-1)),
}

fn can_merge_jobs_with_demand_impl(
    cluster: Option<(i32, i32, i32, i32)>,
    candidate: Option<(i32, i32, i32, i32)>,
    expected: Result<(i32, i32, i32, i32), i32>,
) {
    let create_demand = |demand: (i32, i32, i32, i32)| Demand::<SingleDimLoad> {
        pickup: (SingleDimLoad::new(demand.0), SingleDimLoad::new(demand.1)),
        delivery: (SingleDimLoad::new(demand.2), SingleDimLoad::new(demand.3)),
    };
    let cluster = Job::Single(if let Some(cluster) = cluster {
        TestSingleBuilder::default().demand(create_demand(cluster)).build_shared()
    } else {
        TestSingleBuilder::default().build_shared()
    });
    let candidate = Job::Single(if let Some(candidate) = candidate {
        TestSingleBuilder::default().demand(create_demand(candidate)).build_shared()
    } else {
        TestSingleBuilder::default().build_shared()
    });
    let constraint = create_feature().constraint.unwrap();

    let result: Result<Demand<SingleDimLoad>, ViolationCode> = constraint
        .merge(cluster, candidate)
        .and_then(|job| job.dimens().get_job_demand().cloned().ok_or(ViolationCode::unknown()));

    match (result, expected) {
        (Ok(result), Ok((pickup0, pickup1, delivery0, delivery1))) => {
            assert_eq!(result.pickup.0, SingleDimLoad::new(pickup0));
            assert_eq!(result.pickup.1, SingleDimLoad::new(pickup1));
            assert_eq!(result.delivery.0, SingleDimLoad::new(delivery0));
            assert_eq!(result.delivery.1, SingleDimLoad::new(delivery1));
        }
        (Ok(_), Err(err)) => unreachable!("unexpected ok, when err '{}' expected", err),
        (Err(err), Ok(_)) => unreachable!("unexpected err: '{}'", err),
        (Err(ViolationCode(result)), Err(expected)) => assert_eq!(result, expected),
    }
}
