#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_swap_star_test.rs"]
mod exchange_swap_star_test;

use super::*;
use crate::models::problem::Job;
use crate::solver::search::create_environment_with_custom_quota;
use crate::utils::Either;
use rand::seq::SliceRandom;
use rosomaxa::utils::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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
/// For more details, see `<https://arxiv.org/abs/2012.10384>`
pub struct ExchangeSwapStar {
    leg_selection: LegSelection,
    result_selector: Box<dyn ResultSelector>,
    quota_limit: usize,
}

impl ExchangeSwapStar {
    /// Creates a new instance of `ExchangeSwapStar`.
    pub fn new(random: Arc<dyn Random>, quota_limit: usize) -> Self {
        Self {
            leg_selection: LegSelection::Stochastic(random),
            result_selector: Box::<BestResultSelector>::default(),
            quota_limit,
        }
    }
}

impl LocalOperator for ExchangeSwapStar {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        // NOTE higher value affects performance
        const ROUTE_PAIRS_THRESHOLD: usize = 8;

        let route_pairs = create_route_pairs(insertion_ctx, ROUTE_PAIRS_THRESHOLD);

        // modify environment to include median as an extra quota to prevent long runs
        let limit = refinement_ctx.statistics().speed.get_median().map(|median| median.max(self.quota_limit));
        let mut insertion_ctx = InsertionContext {
            environment: create_environment_with_custom_quota(limit, insertion_ctx.environment.as_ref()),
            ..insertion_ctx.deep_copy()
        };

        let _ = route_pairs.into_iter().try_for_each(|route_pair| {
            let is_quota_reached = try_exchange_jobs_in_routes(
                &mut insertion_ctx,
                route_pair,
                &self.leg_selection,
                self.result_selector.as_ref(),
            );

            if is_quota_reached {
                Err(())
            } else {
                Ok(())
            }
        });

        Some(InsertionContext { environment: refinement_ctx.environment.clone(), ..insertion_ctx })
    }
}

/// Encapsulates common data used by search phase.
type SearchContext<'a> = (&'a InsertionContext, &'a LegSelection, &'a (dyn ResultSelector));

fn get_route_by_idx(insertion_ctx: &InsertionContext, route_idx: usize) -> &RouteContext {
    insertion_ctx.solution.routes.get(route_idx).expect("invalid route index")
}

fn get_movable_jobs(insertion_ctx: &InsertionContext, route_ctx: &RouteContext) -> Vec<Job> {
    route_ctx.route().tour.jobs().filter(|job| !insertion_ctx.solution.locked.contains(*job)).cloned().collect()
}

fn get_evaluation_context<'a>(search_ctx: &'a SearchContext, job: &'a Job) -> EvaluationContext<'a> {
    EvaluationContext {
        goal: search_ctx.0.problem.goal.as_ref(),
        job,
        leg_selection: search_ctx.1,
        result_selector: search_ctx.2,
    }
}

/// Creates route pairs to exchange jobs.
#[allow(clippy::needless_collect)] // NOTE enforce size hint to be non-zero
fn create_route_pairs(insertion_ctx: &InsertionContext, route_pairs_threshold: usize) -> Vec<(usize, usize)> {
    let random = insertion_ctx.environment.random.clone();

    if random.is_hit(0.1) { None } else { group_routes_by_proximity(insertion_ctx) }
        .map(|route_groups_distances| {
            let used_indices = RefCell::new(HashSet::<(usize, usize)>::new());
            let distances = route_groups_distances
                .into_iter()
                .enumerate()
                .flat_map(|(outer_idx, mut route_group_distance)| {
                    let shuffle_amount = (route_group_distance.len() as Float * 0.1) as usize;
                    route_group_distance.partial_shuffle(&mut random.get_rng(), shuffle_amount);
                    route_group_distance
                        .iter()
                        .cloned()
                        .filter(|(inner_idx, _)| {
                            let used_indices = used_indices.borrow();
                            !used_indices.contains(&(outer_idx, *inner_idx))
                                && !used_indices.contains(&(*inner_idx, outer_idx))
                        })
                        .map(|(inner_idx, _)| {
                            let mut used_indices = used_indices.borrow_mut();
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
                        .filter(move |&inner_idx| outer_idx > inner_idx)
                        .map(move |inner_idx| (outer_idx, inner_idx))
                })
                .collect::<Vec<_>>();
            SelectionSamplingIterator::new(all_route_pairs.into_iter(), route_pairs_threshold, random.clone()).collect()
        })
}

/// Finds insertion cost of the existing job in the route.
fn find_insertion_cost(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> InsertionCost {
    route_ctx
        .route()
        .tour
        .index(job)
        .and_then(|idx| {
            assert_ne!(idx, 0);

            let mut route_ctx = route_ctx.deep_copy();
            route_ctx.route_mut().tour.remove(job);
            search_ctx.0.problem.goal.accept_route_state(&mut route_ctx);

            // NOTE This is not the best approach for multi-jobs
            let &(insertion_ctx, leg_selection, result_selector) = search_ctx;
            eval_job_insertion_in_route(
                insertion_ctx,
                &EvaluationContext { goal: insertion_ctx.problem.goal.as_ref(), job, leg_selection, result_selector },
                &route_ctx,
                InsertionPosition::Concrete(idx - 1),
                InsertionResult::make_failure(),
            )
            .try_into()
            .ok()
            .map(|success: InsertionSuccess| success.cost)
        })
        .unwrap_or_default()
}

/// Tries to find insertion cost for `insert_job` in place of `extract_job`.
/// NOTE hard constraints are NOT evaluated.
fn find_in_place_result(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    insert_job: &Job,
    extract_job: &Job,
) -> InsertionResult {
    let insertion_index = route_ctx.route().tour.index(extract_job).expect("cannot find job in route");
    let position = InsertionPosition::Concrete(insertion_index - 1);

    let route_ctx = remove_job_with_copy(search_ctx, extract_job, route_ctx);

    let eval_ctx = get_evaluation_context(search_ctx, insert_job);

    eval_job_insertion_in_route(search_ctx.0, &eval_ctx, &route_ctx, position, InsertionResult::make_failure())
}

fn find_top_results(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    jobs: &[Job],
) -> HashMap<Job, Vec<InsertionResult>> {
    let legs_count = route_ctx.route().tour.legs().count();

    jobs.iter()
        .map(|job| {
            let eval_ctx = get_evaluation_context(search_ctx, job);

            let mut results = (0..legs_count)
                .map(InsertionPosition::Concrete)
                .map(|position| {
                    eval_job_insertion_in_route(
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
                (InsertionResult::Success(left), InsertionResult::Success(right)) => left.cost.cmp(&right.cost),
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
                match search_ctx.2.select_cost(&acc_success.cost, &success.cost) {
                    Either::Left(_) => (acc_idx, acc_result),
                    Either::Right(_) => (idx, result),
                }
            }
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => (acc_idx, acc_result),
            _ => (idx, result),
        });

    if idx == 0 {
        in_place_result
    } else {
        match result {
            InsertionResult::Success(success) => InsertionResult::make_success(
                success.cost.clone(),
                success.job.clone(),
                success.activities.iter().map(|(activity, idx)| (activity.deep_copy(), *idx)).collect(),
                success.actor.clone(),
            ),
            InsertionResult::Failure(_) => InsertionResult::make_failure(),
        }
    }
}

fn remove_job_with_copy(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> RouteContext {
    let mut route_ctx = route_ctx.deep_copy();
    route_ctx.route_mut().tour.remove(job);
    search_ctx.0.problem.goal.accept_route_state(&mut route_ctx);

    route_ctx
}

/// Tries to exchange jobs between two routes.
fn try_exchange_jobs_in_routes(
    insertion_ctx: &mut InsertionContext,
    route_pair: (usize, usize),
    leg_selection: &LegSelection,
    result_selector: &(dyn ResultSelector),
) -> bool {
    let quota = insertion_ctx.environment.quota.clone();
    let is_quota_reached = move || quota.as_ref().map_or(false, |quota| quota.is_reached());

    if is_quota_reached() {
        return true;
    }

    let search_ctx: SearchContext = (insertion_ctx, leg_selection, result_selector);
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
            inner_jobs.iter().map(move |inner_job| (outer_job, inner_job, delta_outer_job_cost.clone()))
        })
        .collect::<Vec<_>>();

    // search phase
    let (outer_best, inner_best, _) = map_reduce(
        job_pairs.as_slice(),
        |(outer_job, inner_job, delta_outer_job_cost)| {
            if is_quota_reached() {
                return (InsertionResult::make_failure(), InsertionResult::make_failure(), InsertionCost::default());
            }

            let delta_inner_job_cost = find_insertion_cost(&search_ctx, inner_job, inner_route_ctx);

            let outer_in_place_result = find_in_place_result(&search_ctx, inner_route_ctx, outer_job, inner_job);
            let inner_in_place_result = find_in_place_result(&search_ctx, outer_route_ctx, inner_job, outer_job);

            let outer_result = choose_best_result(
                &search_ctx,
                outer_in_place_result,
                outer_top_results.get(*outer_job).unwrap().as_slice(),
            );

            let inner_result = choose_best_result(
                &search_ctx,
                inner_in_place_result,
                inner_top_results.get(*inner_job).unwrap().as_slice(),
            );

            let delta_cost = match (&outer_result, &inner_result) {
                (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) => {
                    &outer_success.cost + &inner_success.cost - delta_outer_job_cost - delta_inner_job_cost
                }
                _ => InsertionCost::default(),
            };

            (outer_result, inner_result, delta_cost)
        },
        || (InsertionResult::make_failure(), InsertionResult::make_failure(), InsertionCost::default()),
        |left, right| match left.2.cmp(&right.2) {
            Ordering::Less => left,
            _ => right,
        },
    );

    try_exchange_jobs(insertion_ctx, (outer_best, inner_best), leg_selection, result_selector);

    is_quota_reached()
}

/// Tries to apply insertion results to target insertion context.
fn try_exchange_jobs(
    insertion_ctx: &mut InsertionContext,
    insertion_pair: (InsertionResult, InsertionResult),
    leg_selection: &LegSelection,
    result_selector: &(dyn ResultSelector),
) {
    if let (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) = insertion_pair {
        let constraint = insertion_ctx.problem.goal.clone();

        let outer_job = outer_success.job.clone();
        let inner_job = inner_success.job.clone();

        // remove jobs from results and revaluate them again
        let mut insertion_successes = once((outer_success, inner_job))
            .chain(once((inner_success, outer_job)))
            .filter_map(|(success, job)| {
                let mut route_ctx = insertion_ctx
                    .solution
                    .routes
                    .iter()
                    .find(|route_ctx| route_ctx.route().actor == success.actor)
                    .expect("cannot find route for insertion")
                    .deep_copy();

                // NOTE job can be already removed in in-place case
                let removed_idx = route_ctx.route().tour.index(&job).unwrap_or(usize::MAX);

                route_ctx.route_mut().tour.remove(&job);
                constraint.accept_route_state(&mut route_ctx);

                let position = success.activities.first().unwrap().1;
                let position = if position < removed_idx || position == 0 { position } else { position - 1 };
                let position = InsertionPosition::Concrete(position);

                let search_ctx: SearchContext = (insertion_ctx, leg_selection, result_selector);
                let eval_ctx = get_evaluation_context(&search_ctx, &success.job);
                let alternative = InsertionResult::make_failure();

                eval_job_insertion_in_route(insertion_ctx, &eval_ctx, &route_ctx, position, alternative)
                    .try_into()
                    .ok()
                    .map(|success: InsertionSuccess| (success, Some(route_ctx)))
            })
            .collect::<Vec<_>>();

        if insertion_successes.len() == 2 {
            apply_insertion_with_route(insertion_ctx, insertion_successes.pop().unwrap());
            apply_insertion_with_route(insertion_ctx, insertion_successes.pop().unwrap());
            finalize_insertion_ctx(insertion_ctx);
        }
    }
}
