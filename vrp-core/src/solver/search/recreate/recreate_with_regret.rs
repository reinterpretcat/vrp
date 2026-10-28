#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/recreate/recreate_with_regret_test.rs"]
mod recreate_with_regret_test;

use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::{ConfigurableRecreate, Recreate};
use rosomaxa::utils::{CollectGroupBy, Random};
use std::collections::HashSet;
use std::sync::Arc;

/// A recreate strategy which computes the difference in cost of inserting customer in its
/// best and kth best route, where `k` is a user-defined parameter. Then it inserts the
/// customer with the max difference in its least cost position.
pub struct RecreateWithRegret {
    recreate: ConfigurableRecreate,
}

impl Recreate for RecreateWithRegret {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}

impl RecreateWithRegret {
    /// Creates a new instance of `RecreateWithRegret`.
    pub fn new(min: usize, max: usize, random: Arc<dyn Random>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<AllJobSelector>::default(),
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random.clone()),
                ResultSelection::Stochastic(ResultSelectorProvider::new_default(random)),
                InsertionHeuristic::new(Box::new(RegretInsertionEvaluator::new(min, max))),
            ),
        }
    }
}

struct RegretInsertionEvaluator {
    min: usize,
    max: usize,
    fallback_evaluator: PositionInsertionEvaluator,
}

impl RegretInsertionEvaluator {
    /// Creates a new instance of `RegretInsertionEvaluator`.
    pub fn new(min: usize, max: usize) -> Self {
        assert!(min > 0);
        assert!(min <= max);

        Self { min, max, fallback_evaluator: PositionInsertionEvaluator::default() }
    }
}

impl InsertionEvaluator for RegretInsertionEvaluator {
    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &dyn ResultSelector,
    ) -> InsertionResult {
        let regret_rank = insertion_ctx.environment.random.uniform_int(self.min as i32, self.max as i32) as usize;

        // NOTE no need to proceed with regret, fallback to more performant reducer
        if regret_rank == 1 || jobs.len() == 1 || routes.is_empty() || insertion_ctx.solution.routes.len() < 2 {
            return self.fallback_evaluator.evaluate_all(insertion_ctx, jobs, routes, leg_selection, result_selector);
        }

        let mut results = self
            .fallback_evaluator
            .evaluate_and_collect_all(insertion_ctx, jobs, routes, leg_selection, result_selector)
            .into_iter()
            .filter_map(|result| match result {
                InsertionResult::Success(success) => Some(success),
                _ => None,
            })
            .collect_group_by_key::<Job, InsertionSuccess, _>(|success| success.job.clone())
            .into_values()
            .filter_map(|successes| {
                if successes.len() < regret_rank {
                    return None;
                }

                get_regret(successes, regret_rank, insertion_ctx.solution.routes.len())
            })
            .collect::<Vec<_>>();

        if !results.is_empty() {
            results.sort_by(|a, b| b.0.cmp(&a.0));

            let (_, best_success) = results.swap_remove(0);

            InsertionResult::Success(best_success)
        } else {
            self.fallback_evaluator.evaluate_all(insertion_ctx, jobs, routes, leg_selection, result_selector)
        }
    }
}

fn get_regret(
    mut successes: Vec<InsertionSuccess>,
    regret_rank: usize,
    route_count: usize,
) -> Option<(InsertionCost, InsertionSuccess)> {
    debug_assert!(regret_rank > 1);

    successes.sort_by(|a, b| a.cost.cmp(&b.cost));

    let (_, mut route_results) = successes.into_iter().fold(
        (HashSet::with_capacity(route_count), Vec::default()),
        |(mut actors, mut results), result| {
            if actors.insert(result.actor.clone()) {
                results.push(result);
            }

            (actors, results)
        },
    );

    if regret_rank <= route_results.len() {
        let kth = route_results.swap_remove(regret_rank - 1);
        let best = route_results.swap_remove(0);

        Some((kth.cost - &best.cost, best))
    } else {
        None
    }
}
