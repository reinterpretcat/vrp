//! This module contains a hyper-heuristic logic.

mod dynamic_selective;
pub use self::dynamic_selective::*;

mod static_selective;
pub use self::static_selective::*;

use crate::prelude::*;
use crate::utils::{ParallelismScope, parallel_into_collect};
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;

/// A heuristic operator which is supposed to improve passed solution.
pub trait HeuristicSearchOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs search for a new (better) solution using given one.
    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution;
}

/// A heuristic operator which is supposed to diversify passed solution.
pub trait HeuristicDiversifyOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a diversification of selected solution.
    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution>;
}

/// A collection of heuristic diversification operators.
pub type HeuristicDiversifyOperators<C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// A heuristic operator which intensifies search around a selected solution.
pub trait HeuristicIntensifyOperator {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs an intensification of the selected solution.
    fn intensify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution>;
}

/// A collection of heuristic intensification operators.
pub type HeuristicIntensifyOperators<C, O, S> =
    Vec<Arc<dyn HeuristicIntensifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic: Display {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a new search in the solution space using selected solution.
    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution>;

    /// Performs a new search in the solution space using selected solutions.
    /// As the `search` method requires a mutable reference, implementations of `search_many` is
    /// supposed to run its sub-searches in parallel.
    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;

    /// Performs a diversification of selected solution in order to increase exploration of the solution space.
    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution>;

    /// Performs a diversification of selected solutions in order to increase exploration of the solution space.
    /// As the `diversify` method requires a mutable reference, implementations of `diversify_many` is
    /// supposed to run its logic in parallel for each solution.
    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;

    /// Performs an intensification around the selected solution.
    fn intensify(&self, _: &Self::Context, _: &Self::Solution) -> Vec<Self::Solution>;

    /// Performs an intensification around selected solutions.
    fn intensify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;
}

/// Decides whether to run diversification search.
fn should_diversify<C, O, S>(heuristic_ctx: &C) -> bool
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    let last = heuristic_ctx.statistics().improvement_1000_ratio;
    let global = heuristic_ctx.statistics().improvement_all_ratio;

    let probability = match last {
        _ if last > 0.2 => 0.001,
        _ if last > 0.1 => 0.01,
        _ if last > 0.05 => 0.02,
        _ if global < 0.001 => 0.1,
        _ => 0.05,
    };

    heuristic_ctx.environment().random.is_hit(probability)
}

/// Runs diversification search on the given solution with some probability.
fn diversify_solution<C, O, S>(
    heuristic_ctx: &C,
    solution: &S,
    operators: &[Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>],
) -> Vec<S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    if operators.is_empty() {
        return Vec::new();
    }

    if should_diversify(heuristic_ctx) {
        apply_diversify_operator(heuristic_ctx, solution, operators)
    } else {
        Vec::new()
    }
}

fn apply_diversify_operator<C, O, S>(
    heuristic_ctx: &C,
    solution: &S,
    operators: &[Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>],
) -> Vec<S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    assert!(!operators.is_empty());

    let random = heuristic_ctx.environment().random.as_ref();
    let operator_idx = random.uniform_int(0, operators.len() as i32 - 1) as usize;
    let operator = &operators[operator_idx];

    operator.diversify(heuristic_ctx, solution)
}

/// For each solution, picks an operator with equal probability and runs diversify once.
/// Runs diversification concurrently on the shared scheduler.
fn diversify_solutions<C, O, S>(
    heuristic_ctx: &C,
    solutions: Vec<&S>,
    operators: &[Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>],
) -> Vec<S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    if operators.is_empty() {
        return Vec::new();
    }

    let solutions = solutions.into_iter().filter(|_| should_diversify(heuristic_ctx)).collect::<Vec<_>>();

    parallel_into_collect(solutions, ParallelismScope::Coarse, |solution| {
        apply_diversify_operator(heuristic_ctx, solution, operators)
    })
    .into_iter()
    .flatten()
    .collect()
}

/// Picks one operator with equal probability and runs it on the given solution.
fn intensify_solution<C, O, S>(
    heuristic_ctx: &C,
    solution: &S,
    operators: &[Arc<dyn HeuristicIntensifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>],
) -> Vec<S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    if operators.is_empty() {
        return Vec::new();
    }

    let random = heuristic_ctx.environment().random.as_ref();
    let operator_idx = random.uniform_int(0, operators.len() as i32 - 1) as usize;

    operators[operator_idx].intensify(heuristic_ctx, solution)
}

/// Runs intensification concurrently for each selected solution.
fn intensify_solutions<C, O, S>(
    heuristic_ctx: &C,
    solutions: Vec<&S>,
    operators: &[Arc<dyn HeuristicIntensifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>],
) -> Vec<S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    if operators.is_empty() {
        return Vec::new();
    }

    parallel_into_collect(solutions, ParallelismScope::Coarse, |solution| {
        intensify_solution(heuristic_ctx, solution, operators)
    })
    .into_iter()
    .flatten()
    .collect()
}
