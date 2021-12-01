#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/local/exchange_swap_star_test.rs"]
mod exchange_swap_star_test;

use super::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::utils::{compare_floats, Either};
use crate::utils::{map_reduce, parallel_collect, Random};
use hashbrown::HashMap;
use std::iter::once;

/// Implements a SWAP* algorithm described in "Hybrid Genetic Search for the CVRP:
/// Open-Source Implementation and SWAP* Neighborhood" by Thibaut Vidal.
///
/// The key idea is described by the following theorem:
/// In a best Swap* move between customers v and v0 within routes r and r0 , the new insertion
/// position of v in r0 is either:
/// i) in place of v0 , or
/// ii) among the three best insertion positions in r0 as evaluated prior to the removal of v0.
/// A symmetrical argument holds for the new insertion position of v0 in r.
/// For more details, see https://arxiv.org/abs/2012.10384
pub struct ExchangeSwapStar {
    leg_selector: Box<dyn LegSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
}

impl ExchangeSwapStar {
    /// Creates a new instance of `ExchangeSwapStar`.
    pub fn new(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            leg_selector: Box::new(VariableLegSelector::new(random)),
            result_selector: Box::new(BestResultSelector::default()),
        }
    }
}

impl LocalOperator for ExchangeSwapStar {
    fn explore(
        &self,
        _refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        let mut insertion_ctx = insertion_ctx.deep_copy();
        let route_count = insertion_ctx.solution.routes.len();

        (0..route_count).for_each(|outer_idx| {
            (0..route_count).filter(|&inner_idx| outer_idx != inner_idx).for_each(|inner_idx| {
                let search_ctx = (&insertion_ctx, self.leg_selector.as_ref(), self.result_selector.as_ref());
                let outer_route_ctx = get_route_by_idx(&insertion_ctx, outer_idx);
                let inner_route_ctx = get_route_by_idx(&insertion_ctx, inner_idx);

                // preprocessing phase
                let outer_jobs = get_movable_jobs(&insertion_ctx, outer_route_ctx);
                let inner_jobs = get_movable_jobs(&insertion_ctx, inner_route_ctx);

                let outer_route_results = find_top_results(&search_ctx, inner_route_ctx, outer_jobs.as_slice());
                let inner_route_results = find_top_results(&search_ctx, outer_route_ctx, inner_jobs.as_slice());

                let job_pairs = outer_jobs
                    .iter()
                    .flat_map(|outer_job| {
                        let delta_outer_job_cost_orig = find_insertion_cost(&search_ctx, outer_job, outer_route_ctx);
                        inner_jobs.iter().map(move |inner_job| (outer_job, inner_job, delta_outer_job_cost_orig))
                    })
                    .collect::<Vec<_>>();

                // search phase
                let (outer_best, inner_best, _) = map_reduce(
                    job_pairs.as_slice(),
                    |&(outer_job, inner_job, delta_outer_job_cost_orig)| {
                        let delta_inner_job_cost = find_insertion_cost(&search_ctx, inner_job, inner_route_ctx);

                        let delta_outer_job_insertion_plan =
                            find_in_place_result(&search_ctx, outer_job, inner_job, inner_route_ctx);
                        let delta_inner_job_insertion_plan =
                            find_in_place_result(&search_ctx, inner_job, outer_job, outer_route_ctx);

                        let outer_result = choose_best_result(
                            &search_ctx,
                            delta_outer_job_insertion_plan,
                            outer_route_results.get(outer_job).unwrap().as_slice(),
                        );

                        let inner_result = choose_best_result(
                            &search_ctx,
                            delta_inner_job_insertion_plan,
                            inner_route_results.get(inner_job).unwrap().as_slice(),
                        );

                        let delta_cost = match (&outer_result, &inner_result) {
                            (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) => Some(
                                outer_success.cost + inner_success.cost
                                    - delta_outer_job_cost_orig
                                    - delta_inner_job_cost,
                            ),
                            _ => None,
                        };

                        (outer_result, inner_result, delta_cost)
                    },
                    || (InsertionResult::make_failure(), InsertionResult::make_failure(), None),
                    |left, right| match (&left, &right) {
                        ((_, _, Some(left_cost)), (_, _, Some(right_cost))) => {
                            if *left_cost < *right_cost {
                                left
                            } else {
                                right
                            }
                        }
                        ((_, _, Some(_)), _) => left,
                        _ => right,
                    },
                );

                try_exchange_jobs(&mut insertion_ctx, (outer_best, inner_best));
            });
        });

        Some(insertion_ctx)
    }
}

/// Encapsulates common data used by search phase.
type SearchContext<'a> =
    (&'a InsertionContext, &'a (dyn LegSelector + Send + Sync), &'a (dyn ResultSelector + Send + Sync));

fn get_route_by_idx(insertion_ctx: &InsertionContext, route_idx: usize) -> &RouteContext {
    insertion_ctx.solution.routes.get(route_idx).expect("invalid route index")
}

fn get_movable_jobs(insertion_ctx: &InsertionContext, route_ctx: &RouteContext) -> Vec<Job> {
    route_ctx.route.tour.jobs().filter(|job| !insertion_ctx.solution.locked.contains(job)).collect()
}

/// Finds insertion cost of the existing job in the route.
fn find_insertion_cost(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> Cost {
    let original_costs = route_ctx.get_route_cost();

    let route_ctx = remove_job(search_ctx, job, route_ctx);

    original_costs - route_ctx.get_route_cost()
}

/// Tries to find insertion cost for `insert_job` in place of `extract_job`.
/// NOTE hard constraints are NOT evaluated.
fn find_in_place_result(
    search_ctx: &SearchContext,
    insert_job: &Job,
    extract_job: &Job,
    route_ctx: &RouteContext,
) -> InsertionResult {
    let insertion_index = route_ctx.route.tour.index(extract_job).expect("cannot find job in route");
    let position = InsertionPosition::Concrete(insertion_index);

    let route_ctx = remove_job(search_ctx, extract_job, route_ctx);

    let eval_ctx = EvaluationContext {
        constraint: search_ctx.0.problem.constraint.as_ref(),
        job: insert_job,
        leg_selector: search_ctx.1,
        result_selector: search_ctx.2,
    };

    evaluate_job_insertion_in_route(search_ctx.0, &eval_ctx, &route_ctx, position, InsertionResult::make_failure())
}

fn find_top_results(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    jobs: &[Job],
) -> HashMap<Job, Vec<InsertionResult>> {
    let legs_count = route_ctx.route.tour.legs().count();

    parallel_collect(jobs, |job| {
        let eval_ctx = EvaluationContext {
            constraint: search_ctx.0.problem.constraint.as_ref(),
            job,
            leg_selector: search_ctx.1,
            result_selector: search_ctx.2,
        };

        let mut results = (0..legs_count)
            .map(InsertionPosition::Concrete)
            .map(|position| {
                evaluate_job_insertion_in_route(
                    search_ctx.0,
                    &eval_ctx,
                    route_ctx,
                    position,
                    InsertionResult::make_failure(),
                )
            })
            .collect::<Vec<_>>();

        results.sort_by(|left, right| match (left, right) {
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => Ordering::Less,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => Ordering::Greater,
            (InsertionResult::Failure(_), InsertionResult::Failure(_)) => Ordering::Equal,
            (InsertionResult::Success(left), InsertionResult::Success(right)) => compare_floats(left.cost, right.cost),
        });

        results.truncate(3);

        (job.clone(), results)
    })
    .into_iter()
    .collect()
}

fn choose_best_result(
    search_ctx: &SearchContext,
    in_place_result: InsertionResult,
    top_results: &[InsertionResult],
) -> InsertionResult {
    let failure = InsertionResult::make_failure();

    let (idx, result) = once(&in_place_result).chain(top_results.iter()).enumerate().fold(
        (0, &failure),
        |(acc_idx, acc_result), (idx, result)| match (acc_result, result) {
            (InsertionResult::Success(acc_success), InsertionResult::Success(success)) => {
                match search_ctx.2.select_cost(&acc_success.context, acc_success.cost, success.cost) {
                    Either::Left => (acc_idx, acc_result),
                    Either::Right => (idx, result),
                }
            }
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => (acc_idx, acc_result),
            _ => (idx, result),
        },
    );

    if idx == 0 {
        in_place_result
    } else {
        match result {
            InsertionResult::Success(success) => InsertionResult::Success(InsertionSuccess {
                cost: success.cost,
                job: success.job.clone(),
                activities: success.activities.iter().map(|(activity, idx)| (activity.deep_copy(), *idx)).collect(),
                context: success.context.clone(),
            }),
            InsertionResult::Failure(_) => InsertionResult::make_failure(),
        }
    }
}

fn remove_job(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> RouteContext {
    let mut route_ctx = route_ctx.deep_copy();
    route_ctx.route_mut().tour.remove(job);
    search_ctx.0.problem.constraint.accept_route_state(&mut route_ctx);

    route_ctx
}

fn try_exchange_jobs(insertion_ctx: &mut InsertionContext, insertion_pair: (InsertionResult, InsertionResult)) {
    if let (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) = insertion_pair {
        // TODO need to remove jobs from routes before insertion and retest it again
        apply_insertion(insertion_ctx, outer_success);
        apply_insertion(insertion_ctx, inner_success);
        finalize_insertion_ctx(insertion_ctx);
    }
}
