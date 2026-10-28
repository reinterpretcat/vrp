use super::*;
use crate::construction::heuristics::MoveContext;
use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Cost;
use crate::models::{Feature, FeatureBuilder, FeatureObjective, Goal, GoalContextBuilder};
use rosomaxa::population::Alternative;
use std::cmp::Ordering;

struct StateObjective(usize);

impl FeatureObjective for StateObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution.solution.state.get_value::<(), Vec<Cost>>().unwrap()[self.0]
    }

    fn estimate(&self, _: &MoveContext<'_>) -> Cost {
        Cost::default()
    }
}

fn create_goal() -> GoalContext {
    let features = ["0", "1"]
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            FeatureBuilder::default().with_name(name).with_objective(StateObjective(idx)).build().unwrap()
        })
        .collect::<Vec<Feature>>();

    GoalContextBuilder::with_features(&features)
        .unwrap()
        .set_main_goal(Goal::subset_of(&features, &["0", "1"]).unwrap())
        .add_alternative_goal(Goal::subset_of(&features, &["1", "0"]).unwrap())
        .build()
        .unwrap()
}

fn create_solution(fitness: [Cost; 2]) -> InsertionContext {
    let mut solution = TestInsertionContextBuilder::default().build();
    solution.solution.state.set_value::<(), _>(fitness.to_vec());
    solution
}

#[test]
fn can_apply_configured_alternative_probability_once() {
    let goal = create_goal();
    let left = create_solution([0., 10.]);
    let right = create_solution([1., 0.]);

    let alternative = create_modified_variant(&goal, Arc::new(FakeRandom::new(vec![1], vec![0.5])), 0., 1.);
    let original = create_modified_variant(&goal, Arc::new(FakeRandom::new(vec![], vec![0.])), 0., 0.);

    assert_eq!(alternative.total_order(&left, &right), Ordering::Greater);
    assert_eq!(original.total_order(&left, &right), Ordering::Less);
}

#[test]
fn can_keep_probability_owned_by_maybe_new() {
    let goal = create_goal();
    let left = create_solution([0., 10.]);
    let right = create_solution([1., 0.]);

    let alternative = goal.maybe_new(&FakeRandom::new(vec![1], vec![0.]));
    let original = goal.maybe_new(&FakeRandom::new(vec![], vec![0.5]));

    assert_eq!(alternative.total_order(&left, &right), Ordering::Greater);
    assert_eq!(original.total_order(&left, &right), Ordering::Less);
}
