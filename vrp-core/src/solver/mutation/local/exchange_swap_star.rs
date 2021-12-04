#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/local/exchange_swap_star_test.rs"]
mod exchange_swap_star_test;

use super::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::utils::{compare_floats, Either, SelectionSamplingIterator};
use crate::utils::{map_reduce, Random};
use hashbrown::{HashMap, HashSet};
use rand::seq::SliceRandom;
use std::iter::once;
use std::sync::RwLock;

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
        const ROUTE_PAIRS_THRESHOLD: usize = 32;

        let mut insertion_ctx = insertion_ctx.deep_copy();

        create_route_pairs(&insertion_ctx, ROUTE_PAIRS_THRESHOLD).into_iter().for_each(|route_pair| {
            try_exchange_jobs_in_routes(
                &mut insertion_ctx,
                route_pair,
                self.leg_selector.as_ref(),
                self.result_selector.as_ref(),
            )
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

fn get_evaluation_context<'a>(search_ctx: &'a SearchContext, job: &'a Job) -> EvaluationContext<'a> {
    EvaluationContext {
        constraint: search_ctx.0.problem.constraint.as_ref(),
        job,
        leg_selector: search_ctx.1,
        result_selector: search_ctx.2,
    }
}

/// Creates route pairs to exchange jobs.
#[allow(clippy::needless_collect)] // NOTE enforce size hint to be non-zero
fn create_route_pairs(insertion_ctx: &InsertionContext, route_pairs_threshold: usize) -> Vec<(usize, usize)> {
    let random = insertion_ctx.environment.random.clone();

    if random.is_hit(0.9) { None } else { group_routes_by_proximity(insertion_ctx) }
        .map(|route_groups_distances| {
            let used_indices = RwLock::new(HashSet::<(usize, usize)>::new());
            let distances = route_groups_distances
                .into_iter()
                .enumerate()
                .flat_map(|(outer_idx, mut route_group_distance)| {
                    let shuffle_amount = (route_group_distance.len() as f64 * 0.5) as usize;
                    route_group_distance.partial_shuffle(&mut random.get_rng(), shuffle_amount);
                    route_group_distance
                        .iter()
                        .cloned()
                        .filter(|(inner_idx, _)| {
                            let used_indices = used_indices.read().unwrap();
                            !used_indices.contains(&(outer_idx, *inner_idx))
                                && !used_indices.contains(&(*inner_idx, outer_idx))
                        })
                        .map(|(inner_idx, _)| {
                            let mut used_indices = used_indices.write().unwrap();
                            used_indices.insert((outer_idx, inner_idx));
                            used_indices.insert((inner_idx, outer_idx));
                            inner_idx
                        })
                        .next()
                        .map(|inner_idx| (outer_idx, inner_idx))
                })
                .collect::<Vec<_>>();
            SelectionSamplingIterator::new(distances.into_iter(), route_pairs_threshold, random.clone()).collect()
        })
        .unwrap_or_else(|| {
            let route_count = insertion_ctx.solution.routes.len();
            // NOTE this is needed to have size hint properly set
            let all_route_pairs = (0..route_count)
                .flat_map(move |outer_idx| {
                    (0..route_count)
                        .filter(move |&inner_idx| outer_idx != inner_idx)
                        .map(move |inner_idx| (outer_idx, inner_idx))
                })
                .collect::<Vec<_>>();
            SelectionSamplingIterator::new(all_route_pairs.into_iter(), route_pairs_threshold, random.clone()).collect()
        })
}

/// Finds insertion cost of the existing job in the route.
fn find_insertion_cost(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> Cost {
    let original_costs = route_ctx.get_route_cost();

    let route_ctx = remove_job_with_copy(search_ctx, job, route_ctx);

    original_costs - route_ctx.get_route_cost()
}

/// Tries to find insertion cost for `insert_job` in place of `extract_job`.
/// NOTE hard constraints are NOT evaluated.
fn find_in_place_result(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    insert_job: &Job,
    extract_job: &Job,
) -> InsertionResult {
    let insertion_index = route_ctx.route.tour.index(extract_job).expect("cannot find job in route");
    let position = InsertionPosition::Concrete(insertion_index - 1);

    let route_ctx = remove_job_with_copy(search_ctx, extract_job, route_ctx);

    let eval_ctx = get_evaluation_context(search_ctx, insert_job);

    evaluate_job_insertion_in_route(search_ctx.0, &eval_ctx, &route_ctx, position, InsertionResult::make_failure())
}

fn find_top_results(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    jobs: &[Job],
) -> HashMap<Job, Vec<InsertionResult>> {
    let legs_count = route_ctx.route.tour.legs().count();

    jobs.iter()
        .map(|job| {
            let eval_ctx = get_evaluation_context(search_ctx, job);

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
                (InsertionResult::Success(left), InsertionResult::Success(right)) => {
                    compare_floats(left.cost, right.cost)
                }
            });

            results.truncate(3);

            (job.clone(), results)
        })
        .collect()
}

fn choose_best_result(
    search_ctx: &SearchContext,
    in_place_result: InsertionResult,
    top_results: &[InsertionResult],
) -> InsertionResult {
    let failure = InsertionResult::make_failure();

    let in_place_idx = in_place_result
        .as_success()
        .and_then(|success| success.activities.first())
        .map(|(_, idx)| *idx)
        .unwrap_or(usize::MAX - 1);

    let (idx, result) = once(&in_place_result)
        .chain(top_results.iter().filter(|result| {
            // NOTE exclude results near in place result
            result.as_success().map_or(false, |success| {
                success
                    .activities
                    .first()
                    .map(|(_, idx)| *idx != in_place_idx && *idx != in_place_idx + 1)
                    .unwrap_or(false)
            })
        }))
        .enumerate()
        .fold((0, &failure), |(acc_idx, acc_result), (idx, result)| match (acc_result, result) {
            (InsertionResult::Success(acc_success), InsertionResult::Success(success)) => {
                match search_ctx.2.select_cost(&acc_success.context, acc_success.cost, success.cost) {
                    Either::Left => (acc_idx, acc_result),
                    Either::Right => (idx, result),
                }
            }
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => (acc_idx, acc_result),
            _ => (idx, result),
        });

    if idx == 0 {
        in_place_result
    } else {
        match result {
            InsertionResult::Success(success) => InsertionResult::Success(InsertionSuccess {
                cost: success.cost,
                job: success.job.clone(),
                activities: success.activities.iter().map(|(activity, idx)| (activity.deep_copy(), *idx)).collect(),
                context: success.context.deep_copy(),
            }),
            InsertionResult::Failure(_) => InsertionResult::make_failure(),
        }
    }
}

fn remove_job_with_copy(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> RouteContext {
    let mut route_ctx = route_ctx.deep_copy();
    route_ctx.route_mut().tour.remove(job);
    search_ctx.0.problem.constraint.accept_route_state(&mut route_ctx);

    route_ctx
}

/// Tries to exchange jobs between two routes.
fn try_exchange_jobs_in_routes(
    insertion_ctx: &mut InsertionContext,
    route_pair: (usize, usize),
    leg_selector: &(dyn LegSelector + Send + Sync),
    result_selector: &(dyn ResultSelector + Send + Sync),
) {
    let search_ctx: SearchContext = (insertion_ctx, leg_selector, result_selector);
    let (outer_idx, inner_idx) = route_pair;

    let outer_route_ctx = get_route_by_idx(insertion_ctx, outer_idx);
    let inner_route_ctx = get_route_by_idx(insertion_ctx, inner_idx);

    // preprocessing phase
    let outer_jobs = get_movable_jobs(insertion_ctx, outer_route_ctx);
    let inner_jobs = get_movable_jobs(insertion_ctx, inner_route_ctx);

    let outer_top_results = find_top_results(&search_ctx, inner_route_ctx, outer_jobs.as_slice());
    let inner_top_results = find_top_results(&search_ctx, outer_route_ctx, inner_jobs.as_slice());

    let job_pairs = outer_jobs
        .iter()
        .flat_map(|outer_job| {
            let delta_outer_job_cost = find_insertion_cost(&search_ctx, outer_job, outer_route_ctx);
            inner_jobs.iter().map(move |inner_job| (outer_job, inner_job, delta_outer_job_cost))
        })
        .collect::<Vec<_>>();

    // search phase
    let (outer_best, inner_best, _) = map_reduce(
        job_pairs.as_slice(),
        |&(outer_job, inner_job, delta_outer_job_cost)| {
            let delta_inner_job_cost = find_insertion_cost(&search_ctx, inner_job, inner_route_ctx);

            let outer_in_place_result = find_in_place_result(&search_ctx, inner_route_ctx, outer_job, inner_job);
            let inner_in_place_result = find_in_place_result(&search_ctx, outer_route_ctx, inner_job, outer_job);

            let outer_result = choose_best_result(
                &search_ctx,
                outer_in_place_result,
                outer_top_results.get(outer_job).unwrap().as_slice(),
            );

            let inner_result = choose_best_result(
                &search_ctx,
                inner_in_place_result,
                inner_top_results.get(inner_job).unwrap().as_slice(),
            );

            let delta_cost = match (&outer_result, &inner_result) {
                (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) => {
                    outer_success.cost + inner_success.cost - delta_outer_job_cost - delta_inner_job_cost
                }
                _ => 0.,
            };

            (outer_result, inner_result, delta_cost)
        },
        || (InsertionResult::make_failure(), InsertionResult::make_failure(), 0.),
        |left, right| match compare_floats(left.2, right.2) {
            Ordering::Less => left,
            _ => right,
        },
    );

    try_exchange_jobs(insertion_ctx, (outer_best, inner_best), leg_selector, result_selector);
}

/// Tries to apply insertion results to target insertion context.
fn try_exchange_jobs(
    insertion_ctx: &mut InsertionContext,
    insertion_pair: (InsertionResult, InsertionResult),
    leg_selector: &(dyn LegSelector + Send + Sync),
    result_selector: &(dyn ResultSelector + Send + Sync),
) {
    if let (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) = insertion_pair {
        let constraint = insertion_ctx.problem.constraint.clone();

        let outer_job = outer_success.job.clone();
        let inner_job = inner_success.job.clone();

        // remove jobs from results and revaluate them again
        let mut insertion_results = once((outer_success, inner_job))
            .chain(once((inner_success, outer_job)))
            .map(|(mut success, job)| {
                success.context = success.context.deep_copy();

                // NOTE job can be already removed in in-place case
                let removed_idx = success.context.route.tour.index(&job).unwrap_or(usize::MAX);

                success.context.route_mut().tour.remove(&job);
                constraint.accept_route_state(&mut success.context);

                let position = success.activities.first().unwrap().1;
                let position = if position < removed_idx || position == 0 { position } else { position - 1 };
                let position = InsertionPosition::Concrete(position);

                let search_ctx: SearchContext = (insertion_ctx, leg_selector, result_selector);
                let eval_ctx = get_evaluation_context(&search_ctx, &success.job);
                let alternative = InsertionResult::make_failure();

                evaluate_job_insertion_in_route(insertion_ctx, &eval_ctx, &success.context, position, alternative)
            })
            .filter_map(|result| result.into_success())
            .collect::<Vec<_>>();

        if insertion_results.len() == 2 {
            apply_insertion(insertion_ctx, insertion_results.pop().unwrap());
            apply_insertion(insertion_ctx, insertion_results.pop().unwrap());
            finalize_insertion_ctx(insertion_ctx);
        }
    }
}
