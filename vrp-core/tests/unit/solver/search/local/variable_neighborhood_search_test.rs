use super::*;
use crate::construction::heuristics::*;
use crate::helpers::models::domain::{ProblemBuilder, TestGoalContextBuilder};
use crate::helpers::solver::create_default_refinement_ctx;
use crate::models::{FeatureBuilder, FeatureObjective};
use crate::solver::RefinementContext;
use rosomaxa::HeuristicStatistics;
use rosomaxa::prelude::{Environment, HeuristicSolution, Quota};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

custom_solution_state!(TestCost typeof f64);

struct TestCostObjective;

impl FeatureObjective for TestCostObjective {
    fn fitness(&self, solution: &InsertionContext) -> f64 {
        solution.solution.state.get_test_cost().copied().unwrap_or_default()
    }

    fn estimate(&self, _: &MoveContext<'_>) -> f64 {
        0.
    }
}

struct TestOperator {
    delta: f64,
}

impl LocalOperator for TestOperator {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let mut candidate = insertion_ctx.deep_copy();
        let current = candidate.solution.state.get_test_cost().copied().unwrap_or_default();
        candidate.solution.state.set_test_cost(current + self.delta);
        Some(candidate)
    }
}

struct CountingOperator {
    calls: Arc<AtomicUsize>,
    delta: f64,
}

struct ScriptedOperator {
    calls: AtomicUsize,
    candidates: Vec<(f64, f64)>,
}

struct ReachedQuota;

impl Quota for ReachedQuota {
    fn is_reached(&self) -> bool {
        true
    }
}

impl LocalOperator for CountingOperator {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        self.calls.fetch_add(1, AtomicOrdering::Relaxed);
        TestOperator { delta: self.delta }.explore(refinement_ctx, insertion_ctx)
    }
}

impl LocalOperator for ScriptedOperator {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let (cost, violation) = self.candidates.get(self.calls.fetch_add(1, AtomicOrdering::Relaxed))?;
        let mut candidate = insertion_ctx.deep_copy();
        candidate.solution.state.set_test_cost(*cost).set_relaxed_violation(*violation);

        Some(candidate)
    }
}

fn create_insertion_ctx() -> InsertionContext {
    let goal = TestGoalContextBuilder::empty()
        .add_feature(FeatureBuilder::default().with_name("cost").with_objective(TestCostObjective).build().unwrap())
        .build();
    let problem = Arc::new(ProblemBuilder::default().with_goal(goal).build());
    let mut insertion_ctx = InsertionContext::new_empty(problem, Arc::new(Environment::default()));
    insertion_ctx.solution.state.set_test_cost(10.);
    insertion_ctx
}

#[test]
fn can_restart_after_strict_improvement() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search = VariableNeighborhoodSearch::new(
        vec![Arc::new(TestOperator { delta: 1. }), Arc::new(TestOperator { delta: -1. })],
        2,
    );

    let result = search.explore(&refinement_ctx, &insertion_ctx).unwrap();

    assert_eq!(result.solution.state.get_test_cost(), Some(&8.));
}

#[test]
fn can_stop_after_complete_failed_pass() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search = VariableNeighborhoodSearch::new(
        vec![Arc::new(TestOperator { delta: 0. }), Arc::new(TestOperator { delta: 1. })],
        2,
    );

    assert!(search.explore(&refinement_ctx, &insertion_ctx).is_none());
}

#[test]
fn can_retain_best_feasible_intermediate_without_changing_endpoint() {
    let mut insertion_ctx = create_insertion_ctx();
    insertion_ctx.solution.state.set_relaxed_violation(0.);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search = VariableNeighborhoodSearch::new(
        vec![Arc::new(ScriptedOperator {
            calls: AtomicUsize::new(0),
            candidates: vec![(9., 0.), (8., 0.), (7., 0.01)],
        })],
        3,
    );

    let result = search.explore_with_checkpoint(&refinement_ctx, &insertion_ctx, Some(refinement_ctx.objective()));

    assert_eq!(result.endpoint.unwrap().solution.state.get_test_cost(), Some(&7.));
    assert_eq!(result.feasible_intermediate.unwrap().solution.state.get_test_cost(), Some(&8.));
}

#[test]
fn regular_vnd_does_not_retain_intermediate_solutions() {
    let mut insertion_ctx = create_insertion_ctx();
    insertion_ctx.solution.state.set_relaxed_violation(0.);
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let search = VariableNeighborhoodSearch::new(
        vec![Arc::new(ScriptedOperator { calls: AtomicUsize::new(0), candidates: vec![(9., 0.), (8., 0.01)] })],
        2,
    );

    let result = search.explore_with_checkpoint(&refinement_ctx, &insertion_ctx, None);

    assert_eq!(result.endpoint.unwrap().solution.state.get_test_cost(), Some(&8.));
    assert!(result.feasible_intermediate.is_none());
}

#[test]
fn can_schedule_extended_neighborhood() {
    let statistics = |generation, improvement_1000_ratio| HeuristicStatistics {
        generation,
        improvement_1000_ratio,
        ..HeuristicStatistics::default()
    };

    assert!(should_use_extended_operator(&statistics(10, 0.1)));
    assert!(!should_use_extended_operator(&statistics(11, 0.1)));
    assert!(should_use_extended_operator(&statistics(1002, 0.)));
    assert!(!should_use_extended_operator(&statistics(1003, 0.)));
}

#[test]
fn can_limit_extended_neighborhood_to_one_attempt() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let calls = Arc::new(AtomicUsize::new(0));
    let search = VariableNeighborhoodSearch::new(vec![Arc::new(TestOperator { delta: 0. })], 4)
        .with_extended_operator(Arc::new(CountingOperator { calls: calls.clone(), delta: -1. }));

    let result = search.explore(&refinement_ctx, &insertion_ctx).unwrap();

    assert_eq!(result.solution.state.get_test_cost(), Some(&9.));
    assert_eq!(calls.load(AtomicOrdering::Relaxed), 1);
}

#[test]
fn can_limit_accepted_improvements() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let calls = Arc::new(AtomicUsize::new(0));
    let search =
        VariableNeighborhoodSearch::new(vec![Arc::new(CountingOperator { calls: calls.clone(), delta: -1. })], 3);

    let result = search.explore(&refinement_ctx, &insertion_ctx).unwrap();

    assert_eq!(result.solution.state.get_test_cost(), Some(&7.));
    assert_eq!(calls.load(AtomicOrdering::Relaxed), 3);
}

#[test]
fn can_limit_operator_attempts() {
    let insertion_ctx = create_insertion_ctx();
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let first_calls = Arc::new(AtomicUsize::new(0));
    let second_calls = Arc::new(AtomicUsize::new(0));
    let search = VariableNeighborhoodSearch::new(
        vec![
            Arc::new(CountingOperator { calls: first_calls.clone(), delta: -1. }),
            Arc::new(CountingOperator { calls: second_calls.clone(), delta: -1. }),
        ],
        8,
    )
    .with_operator_attempt_limit(2);

    let result = search.explore(&refinement_ctx, &insertion_ctx).unwrap();

    assert_eq!(result.solution.state.get_test_cost(), Some(&6.));
    assert_eq!(first_calls.load(AtomicOrdering::Relaxed), 2);
    assert_eq!(second_calls.load(AtomicOrdering::Relaxed), 2);
}

#[test]
fn can_stop_on_reached_quota() {
    let mut insertion_ctx = create_insertion_ctx();
    Arc::make_mut(&mut insertion_ctx.environment).quota = Some(Arc::new(ReachedQuota));
    let refinement_ctx = create_default_refinement_ctx(insertion_ctx.problem.clone());
    let calls = Arc::new(AtomicUsize::new(0));
    let search =
        VariableNeighborhoodSearch::new(vec![Arc::new(CountingOperator { calls: calls.clone(), delta: -1. })], 3);

    assert!(search.explore(&refinement_ctx, &insertion_ctx).is_none());
    assert_eq!(calls.load(AtomicOrdering::Relaxed), 0);
}
