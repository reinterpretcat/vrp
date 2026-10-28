#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_swap_star_test.rs"]
mod exchange_swap_star_test;

use super::*;
use crate::models::problem::{Job, Single};
use crate::models::solution::Leg;
use crate::solver::search::create_environment_with_custom_quota;
use rosomaxa::utils::*;
use std::cell::RefCell;
use std::collections::HashSet;

/// Implements a SWAP* algorithm described in "Hybrid Genetic Search for the CVRP:
/// Open-Source Implementation and SWAP* Neighborhood" by Thibaut Vidal.
///
/// Customers are exchanged between routes, but can be reinserted away from the vacated positions.
/// For additive travel costs, the best position is either the new removal gap or one of the three
/// cheapest original insertion positions. With richer features this is a bounded shortlist, not an
/// exact guarantee. Positions are checked against the normal constraints after removing the other job.
/// For more details, see `<https://arxiv.org/abs/2012.10384>`
pub struct ExchangeSwapStar {
    leg_selection: LegSelection,
    result_selector: Box<dyn ResultSelector>,
}

impl ExchangeSwapStar {
    /// Creates a new instance of `ExchangeSwapStar`.
    pub fn new(random: Arc<dyn Random>) -> Self {
        Self { leg_selection: LegSelection::Stochastic(random), result_selector: Box::<BestResultSelector>::default() }
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
        let limit =
            refinement_ctx.statistics().speed.get_median().map(|median| ((median.max(10) as f64) * 1.5) as usize);
        let search_environment = create_environment_with_custom_quota(limit, insertion_ctx.environment.as_ref());
        let quota = search_environment.quota.clone();
        let mut candidate = None;

        let _ = route_pairs.into_iter().try_for_each(|route_pair| {
            let source = candidate.as_ref().unwrap_or(insertion_ctx);
            let (insertion_pair, is_quota_reached) = find_exchange_jobs_in_routes(
                source,
                route_pair,
                &self.leg_selection,
                self.result_selector.as_ref(),
                quota.as_deref(),
            );

            if let Some(insertion_pair) = insertion_pair
                && let Some(mut next) =
                    try_exchange_jobs(source, insertion_pair, &self.leg_selection, self.result_selector.as_ref())
            {
                next.environment = search_environment.clone();
                candidate = Some(next);
            }

            if is_quota_reached { Err(()) } else { Ok(()) }
        });

        candidate.map(|candidate| InsertionContext { environment: refinement_ctx.environment.clone(), ..candidate })
    }
}

/// Encapsulates common data used by search phase.
type SearchContext<'a> = (&'a InsertionContext, &'a LegSelection, &'a dyn ResultSelector);

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
fn create_route_pairs(insertion_ctx: &InsertionContext, route_pairs_threshold: usize) -> Vec<(usize, usize)> {
    let random = insertion_ctx.environment.random.clone();

    if random.is_hit(0.1) {
        let route_count = insertion_ctx.solution.routes.len();
        // NOTE this is needed to have size hint properly set
        let all_route_pairs = (0..route_count)
            .flat_map(move |outer_idx| {
                (0..route_count)
                    .filter(move |&inner_idx| outer_idx > inner_idx)
                    .map(move |inner_idx| (outer_idx, inner_idx))
            })
            .collect::<Vec<_>>();

        SelectionSamplingIterator::new(all_route_pairs.into_iter(), route_pairs_threshold, random).collect()
    } else {
        let route_groups = group_routes_by_proximity(insertion_ctx);
        let used_indices = RefCell::new(HashSet::<(usize, usize)>::new());
        let distances = route_groups
            .into_iter()
            .enumerate()
            .flat_map(|(outer_idx, route_group)| {
                route_group
                    .into_iter()
                    .filter(|inner_idx| {
                        let used_indices = used_indices.borrow();
                        !used_indices.contains(&(outer_idx, *inner_idx))
                            && !used_indices.contains(&(*inner_idx, outer_idx))
                    })
                    .inspect(|inner_idx| {
                        let mut used_indices = used_indices.borrow_mut();
                        used_indices.insert((outer_idx, *inner_idx));
                        used_indices.insert((*inner_idx, outer_idx));
                    })
                    .next()
                    .map(|inner_idx| (outer_idx, inner_idx))
            })
            .collect::<Vec<_>>();

        SelectionSamplingIterator::new(distances.into_iter(), route_pairs_threshold, random).collect()
    }
}

struct JobRemovalContext {
    route_ctx: RouteContext,
    position: InsertionPosition,
    original_cost: InsertionCost,
    removed_indices: Vec<usize>,
}

/// Creates a route context with `extract_job` removed and evaluates its original insertion cost.
fn prepare_job_removal(search_ctx: &SearchContext, route_ctx: &RouteContext, extract_job: &Job) -> JobRemovalContext {
    let removed_indices = route_ctx
        .route()
        .tour
        .all_activities()
        .enumerate()
        .filter_map(|(index, activity)| (activity.retrieve_job().as_ref() == Some(extract_job)).then_some(index))
        .collect();
    let insertion_index = route_ctx.route().tour.index(extract_job).expect("cannot find job in route");
    let position = InsertionPosition::Concrete(insertion_index - 1);
    let route_ctx = remove_job_with_copy(search_ctx, extract_job, route_ctx);
    let original_cost = eval_job_insertion_in_route(
        search_ctx.0,
        &get_evaluation_context(search_ctx, extract_job),
        &route_ctx,
        position,
        InsertionResult::make_failure(),
    )
    .try_into()
    .ok()
    .map(|success: InsertionSuccess| success.cost)
    .unwrap_or_default();

    JobRemovalContext { route_ctx, position, original_cost, removed_indices }
}

impl JobRemovalContext {
    /// Maps a surviving original leg to its position after removing all activities of the job.
    fn map_position(&self, position: usize) -> Option<usize> {
        let removed_before = self.removed_indices.partition_point(|&index| index <= position);
        let touches_removed =
            removed_before.checked_sub(1).is_some_and(|index| self.removed_indices[index] == position)
                || self.removed_indices.get(removed_before) == Some(&(position + 1));
        (!touches_removed).then(|| position - removed_before)
    }
}

/// Ranks insertion positions before the opposite job is removed from the route.
fn find_top_positions(search_ctx: &SearchContext, route_ctx: &RouteContext, jobs: &[Job]) -> Vec<Vec<usize>> {
    jobs.iter()
        .map(|job| {
            let eval_ctx = get_evaluation_context(search_ctx, job);
            let positions = route_ctx.route().tour.legs().filter_map(|leg| match job {
                // Feasibility before removal must not hide positions which become usable after exchange.
                Job::Single(single) => {
                    estimate_insertion_cost(search_ctx, route_ctx, single, leg).map(|cost| (cost, leg.1))
                }
                // Multi jobs need the insertion machinery to position their dependent activities.
                Job::Multi(_) => eval_job_insertion_in_route(
                    search_ctx.0,
                    &eval_ctx,
                    route_ctx,
                    InsertionPosition::Concrete(leg.1),
                    InsertionResult::make_failure(),
                )
                .try_into()
                .ok()
                .map(|success: InsertionSuccess| (success.cost, success.activities[0].1)),
            });
            select_top_positions(positions)
        })
        .collect()
}

/// Keeps the three cheapest positions without sorting all candidates.
fn select_top_positions(positions: impl Iterator<Item = (InsertionCost, usize)>) -> Vec<usize> {
    const LIMIT: usize = 3;
    let mut best: Vec<(InsertionCost, usize)> = Vec::with_capacity(LIMIT);

    for candidate in positions {
        // Keep scan order for equal costs, including repeated positions produced by Multi jobs.
        let index = best.partition_point(|known| known.0 <= candidate.0);
        if index < LIMIT {
            if best.len() == LIMIT {
                best.pop();
            }
            best.insert(index, candidate);
        }
    }

    best.into_iter().map(|(_, index)| index).collect()
}

/// Ranks single-job positions using the configured objective, without claiming they are feasible.
fn estimate_insertion_cost(
    search_ctx: &SearchContext,
    route_ctx: &RouteContext,
    single: &Arc<Single>,
    leg: Leg<'_>,
) -> Option<InsertionCost> {
    let (activities, index) = leg;
    let prev = activities.first()?;
    let next = activities.get(1);
    let start_time = route_ctx.route().tour.start()?.schedule.departure;
    let mut target = Activity::new_with_job(single.clone());
    let mut best = None;

    for (place_idx, place) in single.places.iter().enumerate() {
        target.place.idx = place_idx;
        target.place.location = place.location.unwrap_or(prev.place.location);
        target.place.duration = place.duration;
        for time in &place.times {
            target.place.time = time.to_time_window(start_time);
            let activity_ctx = ActivityContext { index, prev, target: &target, next };
            let cost = search_ctx.0.problem.goal.estimate(&MoveContext::activity(
                &search_ctx.0.solution,
                route_ctx,
                &activity_ctx,
            ));
            if best.as_ref().is_none_or(|best| cost < *best) {
                best = Some(cost);
            }
        }
    }

    best
}

fn find_best_result(
    search_ctx: &SearchContext,
    removal_ctx: &JobRemovalContext,
    insert_job: &Job,
    top_positions: &[usize],
) -> InsertionResult {
    let eval_ctx = get_evaluation_context(search_ctx, insert_job);
    // These positions share the same job and post-removal route, so route costs can be reused.
    let positions = std::iter::once(removal_ctx.position).chain(
        top_positions.iter().filter_map(|&index| removal_ctx.map_position(index).map(InsertionPosition::Concrete)),
    );

    eval_job_insertions_in_route(search_ctx.0, &eval_ctx, &removal_ctx.route_ctx, positions)
}

fn remove_job_with_copy(search_ctx: &SearchContext, job: &Job, route_ctx: &RouteContext) -> RouteContext {
    let mut route_ctx = route_ctx.deep_copy();
    route_ctx.route_mut().tour.remove(job);
    search_ctx.0.problem.goal.accept_route_state(&mut route_ctx);

    route_ctx
}

/// Tries to exchange jobs between two routes.
type InsertionResultPair = (InsertionResult, InsertionResult);

fn find_exchange_jobs_in_routes(
    insertion_ctx: &InsertionContext,
    route_pair: (usize, usize),
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
    quota: Option<&dyn Quota>,
) -> (Option<InsertionResultPair>, bool) {
    let is_quota_reached = || quota.is_some_and(|quota| quota.is_reached());

    if is_quota_reached() {
        return (None, true);
    }

    let search_ctx: SearchContext = (insertion_ctx, leg_selection, result_selector);
    let (outer_idx, inner_idx) = route_pair;

    let outer_route_ctx = get_route_by_idx(insertion_ctx, outer_idx);
    let inner_route_ctx = get_route_by_idx(insertion_ctx, inner_idx);

    // preprocessing phase
    let outer_jobs = get_movable_jobs(insertion_ctx, outer_route_ctx);
    let inner_jobs = get_movable_jobs(insertion_ctx, inner_route_ctx);

    let outer_top_positions = find_top_positions(&search_ctx, inner_route_ctx, outer_jobs.as_slice());
    let inner_top_positions = find_top_positions(&search_ctx, outer_route_ctx, inner_jobs.as_slice());

    // Removing a job, refreshing its route state, and evaluating its original cost depend only on
    // that job and route. Prepare them once for every candidate paired with the extracted job.
    let outer_removal_contexts = outer_jobs
        .iter()
        .map(|outer_job| prepare_job_removal(&search_ctx, outer_route_ctx, outer_job))
        .collect::<Vec<_>>();

    if is_quota_reached() {
        return (None, true);
    }

    let inner_removal_contexts = inner_jobs
        .iter()
        .map(|inner_job| prepare_job_removal(&search_ctx, inner_route_ctx, inner_job))
        .collect::<Vec<_>>();

    if is_quota_reached() {
        return (None, true);
    }

    let mut job_pairs = Vec::with_capacity(outer_jobs.len() * inner_jobs.len());
    (0..outer_jobs.len()).for_each(|outer_idx| {
        job_pairs.extend((0..inner_jobs.len()).map(|inner_idx| (outer_idx, inner_idx)));
    });

    // search phase
    let (outer_best, inner_best, _) = map_reduce(
        job_pairs.as_slice(),
        |(outer_idx, inner_idx)| {
            if is_quota_reached() {
                return (InsertionResult::make_failure(), InsertionResult::make_failure(), InsertionCost::default());
            }

            let outer_job = &outer_jobs[*outer_idx];
            let inner_job = &inner_jobs[*inner_idx];

            let outer_result = find_best_result(
                &search_ctx,
                &inner_removal_contexts[*inner_idx],
                outer_job,
                &outer_top_positions[*outer_idx],
            );
            let inner_result = find_best_result(
                &search_ctx,
                &outer_removal_contexts[*outer_idx],
                inner_job,
                &inner_top_positions[*inner_idx],
            );

            let delta_cost = match (&outer_result, &inner_result) {
                (InsertionResult::Success(outer_success), InsertionResult::Success(inner_success)) => {
                    &outer_success.cost + &inner_success.cost
                        - &outer_removal_contexts[*outer_idx].original_cost
                        - &inner_removal_contexts[*inner_idx].original_cost
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

    let insertion_pair = match (&outer_best, &inner_best) {
        (InsertionResult::Success(_), InsertionResult::Success(_)) => Some((outer_best, inner_best)),
        _ => None,
    };

    (insertion_pair, is_quota_reached())
}

/// Rechecks a shortlisted exchange on a complete candidate. Positions are already relative to removed routes.
fn try_exchange_jobs(
    insertion_ctx: &InsertionContext,
    insertion_pair: (InsertionResult, InsertionResult),
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
) -> Option<InsertionContext> {
    let (InsertionResult::Success(outer), InsertionResult::Success(inner)) = insertion_pair else {
        return None;
    };
    if insertion_ctx.solution.locked.contains(&outer.job) || insertion_ctx.solution.locked.contains(&inner.job) {
        return None;
    }

    // Only the chosen exchange gets a full copy. Losing job pairs use the prepared route contexts.
    let mut candidate = insertion_ctx.deep_copy();
    for (success, removed_job) in [(&outer, &inner.job), (&inner, &outer.job)] {
        let route_ctx = candidate.solution.routes.iter_mut().find(|route| route.route().actor == success.actor)?;
        if !route_ctx.route_mut().tour.remove(removed_job) {
            return None;
        }
        candidate.problem.goal.accept_route_state(route_ctx);
        candidate.solution.required.push(removed_job.clone());
    }
    let get_anchor = |proposal: &InsertionSuccess| {
        let route = candidate.solution.routes.iter().find(|route| route.route().actor == proposal.actor)?;
        Some(route.route().tour.get(proposal.activities.first()?.1)?.job.clone())
    };
    let anchors = [get_anchor(&outer)?, get_anchor(&inner)?];
    // Route refresh alone does not update shared features, such as job-group membership.
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    for (proposal, anchor) in [outer, inner].into_iter().zip(anchors) {
        let route_ctx = candidate.solution.routes.iter().find(|route| route.route().actor == proposal.actor)?;
        // Shared-state refresh can also remove breaks or reloads. Follow the original predecessor,
        // or search again if it disappeared, instead of applying a shifted numeric position.
        let position = match anchor {
            None => InsertionPosition::Concrete(0),
            Some(anchor) => route_ctx
                .route()
                .tour
                .all_activities()
                .position(|activity| activity.job.as_ref().is_some_and(|job| Arc::ptr_eq(job, &anchor)))
                .map(InsertionPosition::Concrete)
                .unwrap_or(InsertionPosition::Any),
        };
        let search_ctx = (&candidate, leg_selection, result_selector);
        let result = eval_job_insertion_in_route(
            &candidate,
            &get_evaluation_context(&search_ctx, &proposal.job),
            route_ctx,
            position,
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return None;
        };
        apply_insertion_success(&mut candidate, success);
    }
    finalize_insertion_ctx(&mut candidate);

    Some(candidate)
}

#[cfg(test)]
fn try_exchange_jobs_in_routes(
    insertion_ctx: &mut InsertionContext,
    route_pair: (usize, usize),
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
) -> bool {
    let quota = insertion_ctx.environment.quota.clone();
    let (insertion_pair, is_quota_reached) =
        find_exchange_jobs_in_routes(insertion_ctx, route_pair, leg_selection, result_selector, quota.as_deref());

    if let Some(insertion_pair) = insertion_pair
        && let Some(candidate) = try_exchange_jobs(insertion_ctx, insertion_pair, leg_selection, result_selector)
    {
        *insertion_ctx = candidate;
    }

    is_quota_reached
}
