use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::search::{ConfigurableRecreate, Recreate};
use crate::solver::RefinementContext;
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
    fn evaluate_job(
        &self,
        insertion_ctx: &InsertionContext,
        job: &Job,
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_job(insertion_ctx, job, routes, leg_selection, result_selector)
    }

    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[&Job],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_route(insertion_ctx, route_ctx, jobs, leg_selection, result_selector)
    }

    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[&Job],
        routes: &[&RouteContext],
        leg_selection: &LegSelection,
        result_selector: &(dyn ResultSelector),
    ) -> InsertionResult {
        let regret_index = insertion_ctx.environment.random.uniform_int(self.min as i32, self.max as i32) as usize;

        // NOTE no need to proceed with regret, fallback to more performant reducer
        if regret_index == 1 || jobs.len() == 1 || routes.is_empty() || insertion_ctx.solution.routes.len() < 2 {
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
            .into_iter()
            .filter_map(|(_, mut successes)| {
                if successes.len() < regret_index {
                    return None;
                }

                successes.sort_by(|a, b| a.cost.cmp(&b.cost));

                let (_, mut job_results) = successes.into_iter().fold(
                    (HashSet::with_capacity(insertion_ctx.solution.routes.len()), Vec::default()),
                    |(mut actors, mut results), result| {
                        if !actors.contains(&result.actor) {
                            results.push(result);
                        } else {
                            actors.insert(result.actor);
                        }

                        (actors, results)
                    },
                );

                if regret_index < job_results.len() {
                    let worst = job_results.swap_remove(regret_index);
                    let best = job_results.swap_remove(0);

                    Some((worst.cost - &best.cost, best))
                } else {
                    None
                }
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
