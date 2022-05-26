use super::*;
use crate::utils::{parallel_into_collect, unwrap_from_result};
use std::cmp::Ordering;
use std::fmt::Formatter;
use std::sync::Arc;

/// A type which specifies probability behavior for heuristic selection.
pub type HeuristicProbability<C, O, S> = (Box<dyn Fn(&C, &S) -> bool + Send + Sync>, PhantomData<O>);

/// A type which specifies a group of multiple heuristic strategies with their probability.
pub type HeuristicSearchGroup<C, O, S> = Vec<(
    Arc<dyn HeuristicSearchOperator<Context = C, Objective = O, Solution = S> + Send + Sync>,
    HeuristicProbability<C, O, S>,
)>;

/// A collection of heuristic diversify operators.
pub type HeuristicDiversifyGroup<C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// A simple hyper-heuristic which selects metaheuristic from the list with fixed (static) probabilities.
pub struct StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    search_group: HeuristicSearchGroup<C, O, S>,
    diversify_group: HeuristicDiversifyGroup<C, O, S>,
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

    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        parallel_into_collect(solutions.iter().enumerate().collect(), |(idx, solution)| {
            heuristic_ctx.environment().parallelism.thread_pool_execute(idx, || {
                unwrap_from_result(
                    self.search_group
                        .iter()
                        .filter(|(_, (probability, _))| probability(heuristic_ctx, solution))
                        // NOTE not more than two search runs in a row
                        .take(2)
                        .try_fold(solution.deep_copy(), |base_solution, (heuristic, _)| {
                            let new_solution = heuristic.search(heuristic_ctx, &base_solution);

                            if heuristic_ctx.objective().total_order(&base_solution, &new_solution) == Ordering::Greater
                            {
                                // NOTE exit immediately as we don't want to lose improvement from original solution
                                Err(new_solution)
                            } else {
                                Ok(new_solution)
                            }
                        }),
                )
            })
        })
    }

    fn diversify(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        diversify_solutions(heuristic_ctx, solutions, self.diversify_group.as_slice())
    }
}

impl<C, O, S> StaticSelective<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `StaticSelective` heuristic.
    pub fn new(search_group: HeuristicSearchGroup<C, O, S>, diversify_group: HeuristicDiversifyGroup<C, O, S>) -> Self {
        assert!(!search_group.is_empty());
        assert!(!diversify_group.is_empty());

        Self { search_group, diversify_group }
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
