use super::*;
use crate::utils::{parallel_into_collect, UnwrapValue};
use std::cmp::Ordering;
use std::fmt::Formatter;
use std::ops::ControlFlow;
use std::sync::Arc;

/// A type which specifies probability behavior for heuristic selection.
pub type HeuristicProbability<C, O, S> = (Box<dyn Fn(&C, &S) -> bool + Send + Sync>, PhantomData<O>);

/// A type which specifies a group of multiple heuristic strategies with their probability.
pub type HeuristicSearchGroup<F, C, O, S> = Vec<(
    Arc<dyn HeuristicSearchOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync>,
    HeuristicProbability<C, O, S>,
)>;

/// A collection of heuristic diversify operators.
pub type HeuristicDiversifyGroup<F, C, O, S> =
    Vec<Arc<dyn HeuristicDiversifyOperator<Fitness = F, Context = C, Objective = O, Solution = S> + Send + Sync>>;

/// A simple hyper-heuristic which selects metaheuristic from the list with fixed (static) probabilities.
pub struct StaticSelective<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    search_group: HeuristicSearchGroup<F, C, O, S>,
    diversify_group: HeuristicDiversifyGroup<F, C, O, S>,
}

impl<F, C, O, S> HyperHeuristic for StaticSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    type Fitness = F;
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn search(&mut self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        vec![self.search_once(heuristic_ctx, solution)]
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        parallel_into_collect(solutions.iter().enumerate().collect(), |(idx, solution)| {
            heuristic_ctx
                .environment()
                .parallelism
                .thread_pool_execute(idx, || self.search_once(heuristic_ctx, solution))
        })
    }

    fn diversify(&self, heuristic_ctx: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        let probability = get_diversify_probability(heuristic_ctx);
        if heuristic_ctx.environment().random.is_hit(probability) {
            diversify_solution(heuristic_ctx, solution, self.diversify_group.as_slice())
        } else {
            Vec::default()
        }
    }

    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        diversify_solutions(heuristic_ctx, solutions, self.diversify_group.as_slice())
    }
}

impl<F, C, O, S> StaticSelective<F, C, O, S>
where
    F: HeuristicFitness,
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    /// Creates a new instance of `StaticSelective` heuristic.
    pub fn new(
        search_group: HeuristicSearchGroup<F, C, O, S>,
        diversify_group: HeuristicDiversifyGroup<F, C, O, S>,
    ) -> Self {
        assert!(!search_group.is_empty());
        assert!(!diversify_group.is_empty());

        Self { search_group, diversify_group }
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

impl<F, C, O, S> Display for StaticSelective<F, C, O, S>
where
    C: HeuristicContext<Fitness = F, Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S, Fitness = F>,
    S: HeuristicSolution<Fitness = F>,
{
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        // NOTE don't do anything at the moment
        Ok(())
    }
}
