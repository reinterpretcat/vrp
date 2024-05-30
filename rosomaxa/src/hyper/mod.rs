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
    /// A heuristic fitness.
    type Fitness: HeuristicFitness;
    /// A heuristic context type.
    type Context: HeuristicContext<Fitness = Self::Fitness, Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Fitness = Self::Fitness, Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs search for a new (better) solution using given one.
    fn search(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Self::Solution;
}

/// A heuristic operator which is supposed to diversify passed solution.
pub trait HeuristicDiversifyOperator {
    /// A heuristic fitness.
    type Fitness: HeuristicFitness;
    /// A heuristic context type.
    type Context: HeuristicContext<Fitness = Self::Fitness, Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Fitness = Self::Fitness, Solution = Self::Solution>;
    /// A heuristic solution type.
    type Solution: HeuristicSolution;

    /// Performs a diversification of selected solution.
    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution>;
}

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic: Display {
    /// A heuristic fitness type.
    type Fitness: HeuristicFitness;
    /// A heuristic context type.
    type Context: HeuristicContext<Fitness = Self::Fitness, Objective = Self::Objective, Solution = Self::Solution>;
    /// A heuristic objective type.
    type Objective: HeuristicObjective<Fitness = Self::Fitness, Solution = Self::Solution>;
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
}

/// Gets probability to run diversify search.
fn get_diversify_probability<F, C, O, S>(heuristic_ctx: &C) -> f64
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    let last = heuristic_ctx.statistics().improvement_1000_ratio;
    let global = heuristic_ctx.statistics().improvement_all_ratio;

    match last {
        _ if last > 0.2 => 0.001,
        _ if last > 0.1 => 0.01,
        _ if last > 0.05 => 0.02,
        _ if global < 0.001 => 0.1,
        _ => 0.05,
    }
}

/// Runs diversification search on given solution with some probability.
fn diversify_solution<F, C, O, S>(
    heuristic_ctx: &C,
    solution: &S,
    operators: &[Arc<
        dyn HeuristicDiversifyOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync,
    >],
) -> Vec<S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Fitness = F, Solution = S>,
    S: HeuristicSolution<Fitness = F>,
{
    assert!(!operators.is_empty());

    let random = heuristic_ctx.environment().random.as_ref();
    let operator_idx = random.uniform_int(0, operators.len() as i32 - 1) as usize;
    let operator = &operators[operator_idx];

    operator.diversify(heuristic_ctx, solution)
}

/// For each solution, picks an operator with equal probability and runs diversify once.
/// Uses parallelism setting to run diversification on thread pool.
fn diversify_solutions<F, C, O, S>(
    heuristic_ctx: &C,
    solutions: Vec<&S>,
    operators: &[Arc<
        dyn HeuristicDiversifyOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync,
    >],
) -> Vec<S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    assert!(!operators.is_empty());

    let random = heuristic_ctx.environment().random.as_ref();
    let probability = get_diversify_probability(heuristic_ctx);

    let solutions = solutions.into_iter().filter(|_| random.is_hit(probability)).collect::<Vec<_>>();

    parallel_into_collect(solutions.iter().enumerate().collect(), |(solution_idx, solution)| {
        heuristic_ctx
            .environment()
            .parallelism
            .thread_pool_execute(solution_idx, || diversify_solution(heuristic_ctx, solution, operators))
    })
    .into_iter()
    .flatten()
    .collect()
}
