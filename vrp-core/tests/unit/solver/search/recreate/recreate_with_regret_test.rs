use super::*;
use crate::helpers::models::problem::TestSingleBuilder;
use crate::helpers::models::solution::test_actor_with_profile;
use crate::models::problem::Actor;
use rosomaxa::prelude::Float;
use std::sync::Arc;

#[test]
fn can_compute_regret_using_distinct_routes() {
    let actor_a = test_actor_with_profile(0);
    let actor_b = test_actor_with_profile(1);
    let successes =
        vec![create_success(1., actor_a.clone()), create_success(100., actor_a.clone()), create_success(5., actor_b)];

    let (regret, best) = get_regret(successes, 2, 2).expect("cannot compute regret");

    assert_eq!(regret, InsertionCost::new(&[4.]));
    assert!(Arc::ptr_eq(&best.actor, &actor_a));
}

#[test]
fn can_compute_regret_when_number_of_routes_equals_rank() {
    let actor_a = test_actor_with_profile(0);
    let actor_b = test_actor_with_profile(1);
    let successes = vec![create_success(1., actor_a.clone()), create_success(5., actor_b)];

    let (regret, best) = get_regret(successes, 2, 2).expect("cannot compute regret");

    assert_eq!(regret, InsertionCost::new(&[4.]));
    assert!(Arc::ptr_eq(&best.actor, &actor_a));
}

fn create_success(cost: Float, actor: Arc<Actor>) -> InsertionSuccess {
    InsertionSuccess {
        cost: InsertionCost::new(&[cost]),
        job: TestSingleBuilder::default().build_as_job_ref(),
        activities: Vec::default(),
        actor,
    }
}
