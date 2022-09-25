use super::*;
use crate::construction::heuristics::{ActivityContext, RouteState};
use crate::helpers::construction::features::*;
use crate::helpers::models::domain::create_empty_solution_context;
use crate::helpers::models::problem::*;
use crate::helpers::models::solution::*;
use crate::models::common::{Demand, DemandDimension, SingleDimLoad};
use crate::models::problem::{Job, Vehicle};
use crate::models::solution::Activity;
use std::sync::Arc;

const STATE_KEY: StateKey = 2;
const VIOLATION_CODE: ViolationCode = 2;

fn create_feature() -> Feature {
    create_capacity_limit_feature::<SingleDimLoad>("capacity", STATE_KEY).unwrap()
}

fn create_test_vehicle(capacity: i32) -> Vehicle {
    VehicleBuilder::default().id("v1").capacity(capacity).build()
}

fn create_constraint_violation(stopped: bool) -> Option<ConstraintViolation> {
    Some(ConstraintViolation { code: VIOLATION_CODE, stopped })
}

fn get_simple_capacity_state(key: i32, state: &RouteState, activity: Option<&Activity>) -> i32 {
    state.get_activity_state::<SingleDimLoad>(key, activity.unwrap()).expect("expect single capacity").value
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
    let mut route_ctx = create_route_context_with_activities(
        &fleet,
        "v1",
        vec![
            test_activity_with_job(test_single_with_simple_demand(create_simple_demand(s1))),
            test_activity_with_job(test_single_with_simple_demand(create_simple_demand(s2))),
            test_activity_with_job(test_single_with_simple_demand(create_simple_demand(s3))),
        ],
    );

    create_feature().state.unwrap().accept_route_state(&mut route_ctx);

    let tour = &route_ctx.route.tour;
    let state = &route_ctx.state;
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, state, tour.start()), start);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, state, tour.end()), end);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, state, tour.get(1)), exp_s1);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, state, tour.get(2)), exp_s2);
    assert_eq!(get_simple_capacity_state(CURRENT_CAPACITY_KEY, state, tour.get(3)), exp_s3);
}

parameterized_test! {can_evaluate_demand_on_route, (size, expected), {
    can_evaluate_demand_on_route_impl(size, expected);
}}

can_evaluate_demand_on_route! {
    case01: (11, Some(ConstraintViolation { code: VIOLATION_CODE, stopped: true })),
    case02: (10, None),
    case03: (9, None),
}

fn can_evaluate_demand_on_route_impl(size: i32, expected: Option<ConstraintViolation>) {
    let fleet = FleetBuilder::default().add_driver(test_driver()).add_vehicle(create_test_vehicle(10)).build();
    let solution_ctx = create_empty_solution_context();
    let route_ctx = create_route_context_with_activities(&fleet, "v1", vec![]);
    let job = Job::Single(test_single_with_simple_demand(create_simple_demand(size)));

    let result = create_feature().constraint.unwrap().evaluate(&MoveContext::route(&solution_ctx, &route_ctx, &job));

    assert_eq!(result, expected);
}

parameterized_test! {can_evaluate_demand_on_activity, (sizes, neighbours, size, expected), {
    can_evaluate_demand_on_activity_impl(sizes, neighbours, size, expected);
}}

can_evaluate_demand_on_activity! {
    case01: (vec![1, 1], (1, 2), 1, None),
    case02: (vec![1, 1], (1, 2), 10, create_constraint_violation(true)),
    case03: (vec![-5, -5], (1, 2), -1, create_constraint_violation(true)),
    case04: (vec![5, 5], (1, 2), 1, create_constraint_violation(true)),
    case05: (vec![-5, 5], (1, 2), 1, None),
    case06: (vec![5, -5], (1, 2), 1, create_constraint_violation(true)),
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
    let mut route_ctx = create_route_context_with_activities(
        &fleet,
        "v1",
        sizes
            .into_iter()
            .map(|size| test_activity_with_job(test_single_with_simple_demand(create_simple_demand(size))))
            .collect(),
    );
    let feature = create_feature();
    feature.state.unwrap().accept_route_state(&mut route_ctx);
    let target = test_activity_with_job(test_single_with_simple_demand(create_simple_demand(size)));
    let activity_ctx = ActivityContext {
        index: 0,
        prev: route_ctx.route.tour.get(neighbours.0).unwrap(),
        target: &target,
        next: route_ctx.route.tour.get(neighbours.1),
    };

    let result = feature.constraint.unwrap().evaluate(&MoveContext::activity(&route_ctx, &activity_ctx));

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
        test_single_with_simple_demand(create_demand(cluster))
    } else {
        Arc::new(test_single())
    });
    let candidate = Job::Single(if let Some(candidate) = candidate {
        test_single_with_simple_demand(create_demand(candidate))
    } else {
        Arc::new(test_single())
    });
    let constraint = create_feature().constraint.unwrap();

    let result: Result<Demand<SingleDimLoad>, i32> =
        constraint.merge(cluster, candidate).and_then(|job| job.dimens().get_demand().cloned().ok_or(-1));

    match (result, expected) {
        (Ok(result), Ok((pickup0, pickup1, delivery0, delivery1))) => {
            assert_eq!(result.pickup.0, SingleDimLoad::new(pickup0));
            assert_eq!(result.pickup.1, SingleDimLoad::new(pickup1));
            assert_eq!(result.delivery.0, SingleDimLoad::new(delivery0));
            assert_eq!(result.delivery.1, SingleDimLoad::new(delivery1));
        }
        (Ok(_), Err(err)) => unreachable!("unexpected ok, when err '{}' expected", err),
        (Err(err), Ok(_)) => unreachable!("unexpected err: '{}'", err),
        (Err(result), Err(expected)) => assert_eq!(result, expected),
    }
}
