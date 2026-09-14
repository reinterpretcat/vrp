use super::*;
use crate::construction::heuristics::RouteContext;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::models::problem::{
    FleetBuilder, TestVehicleBuilder, test_driver, test_vehicle_detail, test_vehicle_with_id,
};
use crate::helpers::models::solution::*;
use crate::models::GoalContextBuilder;
use crate::models::problem::Actor;
use std::cmp::Ordering;
use std::sync::Arc;

fn create_test_insertion_ctx(routes: &[Float]) -> InsertionContext {
    let mut insertion_ctx = TestInsertionContextBuilder::default().build();
    let problem = insertion_ctx.problem.clone();

    routes.iter().for_each(|arrival| {
        let mut route_ctx = RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(problem.fleet.as_ref(), "v1").build())
            .build();
        route_ctx.route_mut().tour.all_activities_mut().last().unwrap().schedule.arrival = *arrival;

        insertion_ctx.solution.routes.push(route_ctx);
    });

    insertion_ctx
}

parameterized_test! {can_properly_estimate_solutions, (left, right, expected), {
    can_properly_estimate_solutions_impl(left, right, expected);
}}

can_properly_estimate_solutions! {
    case_01: (&[10.], &[10.], Ordering::Equal),
    case_02: (&[10.], &[11.], Ordering::Less),
    case_03: (&[10.], &[9.], Ordering::Greater),
    case_04: (&[10.], &[10., 10.], Ordering::Equal),
    case_05: (&[10.], &[10., 9.], Ordering::Greater),
    case_06: (&[10.], &[10., 11.], Ordering::Less),
}

fn can_properly_estimate_solutions_impl(left: &[Float], right: &[Float], expected: Ordering) {
    let left = create_test_insertion_ctx(left);
    let right = create_test_insertion_ctx(right);
    let objective = create_minimize_arrival_time_feature("minimize_arrival").unwrap().objective.unwrap();

    let left = objective.fitness(&left);
    let right = objective.fitness(&right);

    assert_eq!(left.total_cmp(&right), expected);
}

#[test]
fn can_apply_shift_penalty_function() {
    let mut fleet_builder = FleetBuilder::default();
    fleet_builder.add_driver(test_driver());
    fleet_builder.add_vehicle(test_vehicle_with_id("v1"));
    fleet_builder.add_vehicle(test_vehicle_with_id("v2"));
    let fleet = Arc::new(fleet_builder.build());

    let build_route = |vehicle_id: &str| {
        RouteContextBuilder::default()
            .with_route(RouteBuilder::default().with_vehicle(fleet.as_ref(), vehicle_id).build())
            .build()
    };

    let mut insertion_ctx = TestInsertionContextBuilder::default();
    insertion_ctx.with_fleet(fleet.clone());
    insertion_ctx.with_routes(vec![build_route("v1"), build_route("v1"), build_route("v2")]);
    let insertion_ctx = insertion_ctx.build();

    let penalty = Arc::new(|variance: Float| variance * 2.);
    let feature = create_balance_shifts_feature_with_penalty("balance_shifts", penalty).unwrap();
    let objective = feature.objective.unwrap();

    let variance = super::calculate_shift_variance(&insertion_ctx.solution);
    let fitness = objective.fitness(&insertion_ctx);

    assert!((fitness - variance * 2.).abs() < 1e-9);
}

#[test]
fn balance_shifts_objective_prefers_even_distribution() {
    let mut vehicle_one = test_vehicle_with_id("v1");
    vehicle_one.details = vec![test_vehicle_detail(), test_vehicle_detail()];

    let mut vehicle_two = TestVehicleBuilder::default().id("v2").build();
    vehicle_two.details =
        vec![test_vehicle_detail(), test_vehicle_detail(), test_vehicle_detail(), test_vehicle_detail()];

    let mut fleet_builder = FleetBuilder::default();
    fleet_builder.add_driver(test_driver());
    fleet_builder.add_vehicle(vehicle_one);
    fleet_builder.add_vehicle(vehicle_two);
    let fleet = Arc::new(fleet_builder.build());

    let mut actors_by_vehicle: HashMap<String, Vec<Arc<Actor>>> = HashMap::new();
    fleet.actors.iter().cloned().for_each(|actor| {
        let vehicle_id = actor.vehicle.dimens.get_vehicle_id().unwrap().clone();
        actors_by_vehicle.entry(vehicle_id).or_default().push(actor);
    });

    let make_route = |actor: Arc<Actor>| RouteContext::new(actor);

    let balanced_routes = vec![
        make_route(actors_by_vehicle.get("v1").unwrap()[0].clone()),
        make_route(actors_by_vehicle.get("v2").unwrap()[0].clone()),
        make_route(actors_by_vehicle.get("v2").unwrap()[1].clone()),
    ];

    let unbalanced_routes = vec![
        make_route(actors_by_vehicle.get("v1").unwrap()[0].clone()),
        make_route(actors_by_vehicle.get("v1").unwrap()[1].clone()),
        make_route(actors_by_vehicle.get("v2").unwrap()[0].clone()),
    ];

    let mut balanced_ctx_builder = TestInsertionContextBuilder::default();
    balanced_ctx_builder.with_fleet(fleet.clone());
    balanced_ctx_builder.with_routes(balanced_routes);
    let balanced_ctx = balanced_ctx_builder.build();

    let mut unbalanced_ctx_builder = TestInsertionContextBuilder::default();
    unbalanced_ctx_builder.with_fleet(fleet);
    unbalanced_ctx_builder.with_routes(unbalanced_routes);
    let unbalanced_ctx = unbalanced_ctx_builder.build();

    let feature = create_balance_shifts_feature("balance").unwrap();
    let goal = GoalContextBuilder::with_features(&[feature]).and_then(|builder| builder.build()).unwrap();

    assert_eq!(goal.total_order(&balanced_ctx, &unbalanced_ctx), Ordering::Less);
}
