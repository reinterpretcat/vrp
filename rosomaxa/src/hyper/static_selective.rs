#[cfg(test)]
#[path = "../../tests/unit/hyper/static_selective_test.rs"]
mod static_selective_test;

use super::*;
use crate::utils::{ParallelismScope, UnwrapValue, parallel_into_collect};
use std::cmp::Ordering;
use std::fmt::Formatter;
use std::ops::ControlFlow;
use std::sync::Arc;

/// A type which specifies probability behavior for heuristic selection.
pub type HeuristicProbability<C, O, S> = (Box<dyn Fn(&C, &S) -> bool + Send + Sync>, PhantomData<O>);

/// A type which specifies a group of multiple heuristic strategies with their probability.
pub type HeuristicSearchGroup<C, O, S> = Vec<(
    Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>,
    HeuristicProbability<C, O, S>,
)>;

/// A collection of heuristic diversification operators.
pub type HeuristicDiversifyGroup<C, O, S> = HeuristicDiversifyOperators<C, O, S>;

/// A simple hyper-heuristic which selects metaheuristic from the list with fixed (static) probabilities.
pub struct StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    search_group: HeuristicSearchGroup<C, O, S>,
    diversify_operators: HeuristicDiversifyOperators<C, O, S>,
    intensify_operators: HeuristicIntensifyOperators<C, O, S>,
}

impl<C, O, S> HyperHeuristic for StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        vec![self.search_once(heuristic_ctx, solution)]
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        parallel_into_collect(solutions, ParallelismScope::Coarse, |solution| self.search_once(heuristic_ctx, solution))
    }

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        diversify_solution(heuristic_ctx, solution, self.diversify_operators.as_slice())
    }

    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        diversify_solutions(heuristic_ctx, solutions, self.diversify_operators.as_slice())
    }

    fn intensify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        intensify_solution(heuristic_ctx, solution, self.intensify_operators.as_slice())
    }

    fn intensify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        intensify_solutions(heuristic_ctx, solutions, self.intensify_operators.as_slice())
    }
}

impl<C, O, S> StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `StaticSelective` heuristic.
    pub fn new(search_group: HeuristicSearchGroup<C, O, S>) -> Self {
        assert!(!search_group.is_empty());

        Self { search_group, diversify_operators: Vec::new(), intensify_operators: Vec::new() }
    }

    /// Adds operators which diversify search during exploration.
    pub fn with_diversify_operators(mut self, operators: HeuristicDiversifyOperators<C, O, S>) -> Self {
        self.diversify_operators = operators;
        self
    }

    /// Adds operators which intensify search during exploitation.
    pub fn with_intensify_operators(mut self, operators: HeuristicIntensifyOperators<C, O, S>) -> Self {
        self.intensify_operators = operators;
        self
    }

    fn search_once(&self, heuristic_ctx: &C, solution: &S) -> S {
        self.search_group
            .iter()
            .filter(|(_, (probability, _))| probability(heuristic_ctx, solution))
            // NOTE not more than two search runs in a row
            .take(2)
            .try_fold(solution.deep_copy(), |base_solution, (heuristic, _)| {
                let new_solution = heuristic.search(heuristic_ctx, &base_solution);

                if heuristic_ctx.objective().total_order(&base_solution, &new_solution) == Ordering::Greater {
                    // NOTE exit immediately as we don't want to lose improvement from original solution
                    ControlFlow::Break(new_solution)
                } else {
                    ControlFlow::Continue(new_solution)
                }
            })
            .unwrap_value()
    }
}

impl<C, O, S> Display for StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        // NOTE don't do anything at the moment
        Ok(())
    }
}
