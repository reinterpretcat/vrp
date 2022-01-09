use super::*;
use crate::algorithms::nsga2::Objective;
use crate::utils::{parallel_into_collect, unwrap_from_result};
use std::cmp::Ordering;
use std::sync::Arc;

/// A type which specifies probability behavior for heuristic selection.
pub type HeuristicProbability<C, O, P, S> = (Box<dyn Fn(&C, &S) -> bool + Send + Sync>, PhantomData<O>, PhantomData<P>);

/// A type which specifies a group of multiple heuristic strategies with their probability.
pub type HeuristicGroup<C, O, P, S> = (
    Vec<(Arc<dyn HeuristicOperator<Context = C, Solution = S> + Send + Sync>, HeuristicProbability<C, O, P, S>)>,
    PhantomData<P>,
);

/// A simple hyper-heuristic which selects metaheuristic from the list with fixed (static) probabilities.
pub struct StaticSelective<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    heuristic_group: HeuristicGroup<C, O, P, S>,
}

impl<C, O, P, S> HyperHeuristic for StaticSelective<C, O, P, S>
where
    C: HeuristicContext<Population = P, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    P: HeuristicPopulation<Objective = O, Individual = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        parallel_into_collect(solutions.iter().enumerate().collect(), |(idx, solution)| {
            heuristic_ctx.environment().parallelism.thread_pool_execute(idx, || {
                unwrap_from_result(
                    self.heuristic_group
                        .0
                        .iter()
                        .filter(|(_, (probability, _, _))| probability(heuristic_ctx, solution))
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
}
