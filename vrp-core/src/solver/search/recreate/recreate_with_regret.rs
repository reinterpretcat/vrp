use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::problem::Job;
use crate::solver::search::{ConfigurableRecreate, Recreate};
use crate::solver::RefinementContext;
use hashbrown::HashSet;
use rosomaxa::utils::{CollectGroupBy, Random};
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
    pub fn new(min: usize, max: usize, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(AllJobSelector::default()),
                Box::new(AllRouteSelector::default()),
                Box::new(VariableLegSelector::new(random)),
                Box::new(BestResultSelector::default()),
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
        routes: &[RouteContext],
        leg_selector: &(dyn LegSelector + Send + Sync),
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_job(insertion_ctx, job, routes, leg_selector, result_selector)
    }

    fn evaluate_route(
        &self,
        insertion_ctx: &InsertionContext,
        route_ctx: &RouteContext,
        jobs: &[Job],
        leg_selector: &(dyn LegSelector + Send + Sync),
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        self.fallback_evaluator.evaluate_route(insertion_ctx, route_ctx, jobs, leg_selector, result_selector)
    }

    fn evaluate_all(
        &self,
        insertion_ctx: &InsertionContext,
        jobs: &[Job],
        routes: &[RouteContext],
        leg_selector: &(dyn LegSelector + Send + Sync),
        result_selector: &(dyn ResultSelector + Send + Sync),
    ) -> InsertionResult {
        let regret_index = insertion_ctx.environment.random.uniform_int(self.min as i32, self.max as i32) as usize;

        // NOTE no need to proceed with regret, fallback to more performant reducer
        if regret_index == 1 || jobs.len() == 1 || routes.is_empty() || insertion_ctx.solution.routes.len() < 2 {
            return self.fallback_evaluator.evaluate_all(insertion_ctx, jobs, routes, leg_selector, result_selector);
        }

        let mut results = self
            .fallback_evaluator
            .evaluate_and_collect_all(insertion_ctx, jobs, routes, leg_selector, result_selector)
            .into_iter()
            .filter_map(|result| match result {
                InsertionResult::Success(success) => Some(success),
                _ => None,
            })
            .collect_group_by_key::<Job, InsertionSuccess, _>(|success| success.job.clone())
            .into_iter()
            .filter_map(|(_, mut success)| {
                if success.len() < regret_index {
                    return None;
                }

                success.sort_by(|a, b| a.cost.cmp(&b.cost));

                let (_, mut job_results) = success.into_iter().fold(
                    (HashSet::with_capacity(insertion_ctx.solution.routes.len()), Vec::default()),
                    |(mut routes, mut results), result| {
                        if !routes.contains(&result.context.route.actor) {
                            results.push(result);
                        } else {
                            routes.insert(result.context.route.actor.clone());
                        }

                        (routes, results)
                    },
                );

                if regret_index < job_results.len() {
                    let worst = job_results.swap_remove(regret_index);
                    let best = job_results.swap_remove(0);

                    Some((worst.cost - best.cost.clone(), best))
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
            self.fallback_evaluator.evaluate_all(insertion_ctx, jobs, routes, leg_selector, result_selector)
        }
    }
}
