use super::*;
use crate::construction::features::{CapacityFeatureBuilder, TransportFeatureBuilder, VehicleCapacityDimension};
use crate::helpers::construction::features::create_simple_demand;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::solver::{
    create_default_refinement_ctx, generate_matrix_routes, generate_matrix_routes_with_defaults, promote_to_locked,
    rearrange_jobs_in_routes,
};
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::ViolationCode;
use crate::models::common::SingleDimLoad;
use rosomaxa::prelude::HeuristicObjective;
use std::sync::Arc;

fn create_insertion_ctx(job_order: &[Vec<&str>]) -> InsertionContext {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 128])));
    let (problem, solution) = generate_matrix_routes_with_defaults(4, 2, true);
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order);

    insertion_ctx
}

fn create_capacity_insertion_ctx(job_order: &[Vec<&str>]) -> InsertionContext {
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 128])));
    let (problem, solution) = generate_matrix_routes(
        4,
        2,
        true,
        |transport, activity, _| {
            TestGoalContextBuilder::empty()
                .add_feature(
                    TransportFeatureBuilder::new("transport")
                        .set_violation_code(ViolationCode(1))
                        .set_transport_cost(transport)
                        .set_activity_cost(activity)
                        .build_minimize_cost()
                        .unwrap(),
                )
                .add_feature(
                    CapacityFeatureBuilder::<SingleDimLoad>::new("capacity")
                        .set_violation_code(ViolationCode(2))
                        .build()
                        .unwrap(),
                )
                .build()
        },
        |id, location| {
            let demand = match id {
                "c0" | "c2" | "c7" => 1,
                "c1" | "c3" | "c5" => 2,
                "c4" => 3,
                "c6" => 6,
                _ => unreachable!(),
            };
            TestSingleBuilder::default().id(id).location(location).demand(create_simple_demand(demand)).build_shared()
        },
        |mut vehicle| {
            vehicle.dimens.set_vehicle_capacity(SingleDimLoad::new(10));
            vehicle
        },
        |data| (data.clone(), data),
    );
    let mut insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order);

    insertion_ctx
}

#[test]
fn can_improve_solution_by_exchanging_route_tails() {
    let insertion_ctx = create_insertion_ctx(&[vec!["c0", "c1", "c6", "c7"], vec!["c4", "c5", "c2", "c3"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = ExchangeTwoOptStar::default().explore(&refinement_ctx, &insertion_ctx).expect("no improvement");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    assert_eq!(
        get_customer_ids_from_routes(&result),
        vec![
            vec!["c0".to_string(), "c1".to_string(), "c2".to_string(), "c3".to_string()],
            vec!["c4".to_string(), "c5".to_string(), "c6".to_string(), "c7".to_string()]
        ]
    );
}

#[test]
fn does_not_move_locked_tail() {
    let insertion_ctx = promote_to_locked(
        create_insertion_ctx(&[vec!["c0", "c1", "c6", "c7"], vec!["c4", "c5", "c2", "c3"]]),
        &["c2", "c3", "c6", "c7"],
    );
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(ExchangeTwoOptStar::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn does_not_return_non_improving_tail_exchange() {
    let insertion_ctx = create_insertion_ctx(&[vec!["c0", "c1", "c2", "c3"], vec!["c4", "c5", "c6", "c7"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    assert!(ExchangeTwoOptStar::default().explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn can_skip_infeasible_reconnection() {
    // The best transport reconnection would create [c6, c4, c5] with load 11. The next candidate
    // keeps both routes within capacity 10 and still reduces transport cost.
    let insertion_ctx = create_capacity_insertion_ctx(&[vec!["c0", "c1", "c4", "c5"], vec!["c6", "c2", "c7", "c3"]]);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());

    let result = ExchangeTwoOptStar::default().explore(&refinement_ctx, &insertion_ctx).expect("no feasible fallback");

    assert_eq!(insertion_ctx.problem.goal.total_order(&result, &insertion_ctx), Ordering::Less);
    assert_eq!(
        get_customer_ids_from_routes(&result),
        vec![
            vec![
                "c0".to_string(),
                "c1".to_string(),
                "c4".to_string(),
                "c2".to_string(),
                "c7".to_string(),
                "c3".to_string()
            ],
            vec!["c6".to_string(), "c5".to_string()]
        ]
    );
}
