//! This module contains a hyper-heuristic logic.

mod dynamic_selective;
pub use self::dynamic_selective::*;

mod static_selective;
pub use self::static_selective::*;

use crate::prelude::*;
use crate::utils::parallel_into_collect;
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

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic: Display {
    /// A heuristic context type.
    type Context: HeuristicContext<Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a new search in the solution space using selected solutions.
    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;

    /// Performs a diversification of selected solutions in order to increase exploration
    /// of the solution space.
    fn diversify(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution>;
}

/// For each solution, picks an operator with equal probability and runs diversify once.
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
    assert!(!operators.is_empty());

    let x = heuristic_ctx.statistics().improvement_1000_ratio;
    let probability = match x {
        _ if x > 0.2 => 0.001,
        _ if x > 0.1 => 0.01,
        _ => 0.05,
    };

    let random = heuristic_ctx.environment().random.as_ref();
    if random.is_hit(probability) {
        parallel_into_collect(solutions.iter().enumerate().collect(), |(solution_idx, solution)| {
            heuristic_ctx.environment().parallelism.thread_pool_execute(solution_idx, || {
                let operator_idx = random.uniform_int(0, operators.len() as i32 - 1) as usize;
                let operator = &operators[operator_idx];

                operator.diversify(heuristic_ctx, solution)
            })
        })
        .into_iter()
        .flatten()
        .collect()
    } else {
        Vec::default()
    }
}
