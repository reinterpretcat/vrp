#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_two_opt_star_test.rs"]
mod exchange_two_opt_star_test;

use crate::construction::heuristics::*;
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{Job, TravelTime};
use crate::models::solution::{Activity, Route};
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use rosomaxa::prelude::{Float, HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;
use std::collections::HashSet;

// Route-only feasibility checks are much cheaper than complete solution copies, but still scale with
// tail length. Keep this as a small fixed implementation budget rather than another runtime knob.
const SCREEN_CANDIDATE_THRESHOLD: usize = 8;
// Relaxed repair needs broader route-pair coverage than ordinary descent. Bound the expensive
// route-state previews while retaining substantially more alternatives than the exact screen.
const RELAXED_PREVIEW_CANDIDATE_THRESHOLD: usize = 256;

/// A granular 2-opt* operator which exchanges ordered tails between nearby routes.
///
/// Candidate cut edges are built from the problem's nearest-job index and ranked using the transport
/// delta of reconnecting the two prefixes to the opposite tails. The best few candidates are screened
/// using copies of only their two routes. During relaxed repair, the screen considers a wider bounded
/// set and ranks it by remaining route-decomposable violation before transport cost. The first ranked
/// candidate is materialized by copying the complete solution and exchanging both tails through the normal
/// constraint pipeline. Relaxed search can try the remaining screened candidates when the complete goal
/// rejects the first proposal.
///
/// The neighbourhood is deliberately bounded. `neighbor_threshold` limits how many neighbours are
/// inspected for each cut job, and `max_tail_jobs` prevents a single operator call from rebuilding very
/// long routes. Locked jobs are never moved, and only a strict lexicographic improvement is returned.
pub struct ExchangeTwoOptStar {
    neighbor_threshold: usize,
    max_tail_jobs: usize,
}

impl ExchangeTwoOptStar {
    /// Creates a new `ExchangeTwoOptStar` instance.
    pub fn new(neighbor_threshold: usize, max_tail_jobs: usize) -> Self {
        assert!(neighbor_threshold > 0);
        assert!(max_tail_jobs > 0);

        Self { neighbor_threshold, max_tail_jobs }
    }
}

impl Default for ExchangeTwoOptStar {
    fn default() -> Self {
        Self::new(32, 16)
    }
}

impl LocalOperator for ExchangeTwoOptStar {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if insertion_ctx.solution.routes.len() < 2 {
            return None;
        }

        select_tail_exchanges(insertion_ctx, self.neighbor_threshold, self.max_tail_jobs)
            .into_iter()
            .filter_map(|exchange| exchange_tails(insertion_ctx, exchange))
            .find(|candidate| insertion_ctx.problem.goal.total_order(candidate, insertion_ctx) == Ordering::Less)
    }
}

struct TailExchange {
    first_route_idx: usize,
    second_route_idx: usize,
    first_position: usize,
    second_position: usize,
}

struct JobPosition {
    route_idx: usize,
    position: usize,
}

#[derive(Clone)]
struct TailExchangeCandidate {
    first_route_idx: usize,
    first_position: usize,
    second_route_idx: usize,
    second_position: usize,
    estimated_cost: Cost,
    violation_excess: Float,
}

fn select_tail_exchanges(
    insertion_ctx: &InsertionContext,
    neighbor_threshold: usize,
    max_tail_jobs: usize,
) -> Vec<TailExchange> {
    let ordered_routes =
        insertion_ctx.solution.routes.iter().map(|route_ctx| get_ordered_jobs(route_ctx.route())).collect::<Vec<_>>();
    let job_positions = ordered_routes
        .iter()
        .enumerate()
        .flat_map(|(route_idx, jobs)| {
            jobs.iter().enumerate().map(move |(position, job)| (job.clone(), JobPosition { route_idx, position }))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let locked = &insertion_ctx.solution.locked;
    let route_violations = insertion_ctx.problem.goal.get_active_relaxed_route_violations(&insertion_ctx.solution);
    let candidate_neighbor_threshold = route_violations.as_ref().map_or(1, |_| neighbor_threshold);
    let mut used = HashSet::new();
    let mut candidates = Vec::new();

    for (first_route_idx, first_jobs) in ordered_routes.iter().enumerate() {
        let profile = &insertion_ctx.solution.routes[first_route_idx].route().actor.vehicle.profile;

        for (first_position, first_job) in first_jobs.iter().enumerate() {
            let first_tail = &first_jobs[first_position + 1..];
            if first_tail.is_empty()
                || first_tail.len() > max_tail_jobs
                || first_tail.iter().any(|job| locked.contains(job))
            {
                continue;
            }

            let second = insertion_ctx
                .problem
                .jobs
                .neighbors(profile, first_job, Timestamp::default())
                .take(neighbor_threshold)
                .filter_map(|(job, _)| job_positions.get(job).map(|position| (job, position)))
                .filter(|(_, position)| {
                    if position.route_idx == first_route_idx {
                        return false;
                    }

                    let second_jobs = &ordered_routes[position.route_idx];
                    let second_tail = &second_jobs[position.position + 1..];
                    !second_tail.is_empty()
                        && second_tail.len() <= max_tail_jobs
                        && !second_tail.iter().any(|job| locked.contains(job))
                })
                .take(candidate_neighbor_threshold);

            for (second_job, second_position) in second {
                if route_violations.as_ref().is_some_and(|violations| {
                    violations[first_route_idx] == 0. && violations[second_position.route_idx] == 0.
                }) {
                    continue;
                }

                let key = if first_route_idx < second_position.route_idx {
                    (first_route_idx, first_position, second_position.route_idx, second_position.position)
                } else {
                    (second_position.route_idx, second_position.position, first_route_idx, first_position)
                };
                if !used.insert(key) {
                    continue;
                }

                let Some(cost) = estimate_exchange_cost(
                    insertion_ctx,
                    first_route_idx,
                    first_job,
                    second_position.route_idx,
                    second_job,
                ) else {
                    continue;
                };
                candidates.push(TailExchangeCandidate {
                    first_route_idx,
                    first_position,
                    second_route_idx: second_position.route_idx,
                    second_position: second_position.position,
                    estimated_cost: cost,
                    violation_excess: 0.,
                });
            }

            if route_violations.is_some() && candidates.len() >= RELAXED_PREVIEW_CANDIDATE_THRESHOLD * 2 {
                retain_best_transport_candidates(&mut candidates, RELAXED_PREVIEW_CANDIDATE_THRESHOLD);
            }
        }
    }

    let is_relaxed = insertion_ctx.problem.goal.is_relaxed();
    if route_violations.is_some() {
        candidates = rank_relaxed_candidates(insertion_ctx, &ordered_routes, candidates, 1);
    } else {
        candidates.sort_unstable_by(|left, right| left.estimated_cost.total_cmp(&right.estimated_cost));

        // Close to feasibility, exact route previews of the short transport screen prevent an excessive cheapest
        // proposal from hiding a valid runner-up. Complete-goal checks below still remain authoritative.
        if is_relaxed {
            candidates.truncate(SCREEN_CANDIDATE_THRESHOLD);
            let transport_ranked = candidates.clone();
            let ranked =
                rank_relaxed_candidates(insertion_ctx, &ordered_routes, candidates, SCREEN_CANDIDATE_THRESHOLD);
            candidates = if ranked.is_empty() { transport_ranked } else { ranked };
        }
    }
    let needs_route_screen = !is_relaxed && route_violations.is_none();

    candidates
        .into_iter()
        .take(SCREEN_CANDIDATE_THRESHOLD)
        .filter(|candidate| !needs_route_screen || is_route_locally_feasible(insertion_ctx, &ordered_routes, candidate))
        .map(|candidate| TailExchange {
            first_route_idx: candidate.first_route_idx,
            second_route_idx: candidate.second_route_idx,
            first_position: candidate.first_position,
            second_position: candidate.second_position,
        })
        // Strict search keeps its existing single completed proposal. Relaxed search has a bounded fallback when a
        // route-level preview cannot represent a solution-level constraint or objective.
        .take(if is_relaxed { SCREEN_CANDIDATE_THRESHOLD } else { 1 })
        .collect()
}

fn rank_relaxed_candidates(
    insertion_ctx: &InsertionContext,
    ordered_routes: &[Vec<Job>],
    mut candidates: Vec<TailExchangeCandidate>,
    band_candidate_limit: usize,
) -> Vec<TailExchangeCandidate> {
    retain_best_transport_candidates(&mut candidates, RELAXED_PREVIEW_CANDIDATE_THRESHOLD);
    candidates.sort_unstable_by(|left, right| left.estimated_cost.total_cmp(&right.estimated_cost));
    let mut evaluated = Vec::with_capacity(candidates.len());

    let mut admissible_count = 0;
    for mut candidate in candidates {
        let Some((first, second)) = create_candidate_routes(insertion_ctx, ordered_routes, &candidate) else {
            continue;
        };
        let current = [
            &insertion_ctx.solution.routes[candidate.first_route_idx],
            &insertion_ctx.solution.routes[candidate.second_route_idx],
        ];
        let replacement = [&first, &second];
        let Some(violation_excess) = insertion_ctx.problem.goal.estimate_relaxed_route_violation_excess(
            &insertion_ctx.solution,
            &current,
            &replacement,
        ) else {
            continue;
        };
        candidate.violation_excess = violation_excess;
        evaluated.push(candidate);

        // Candidates are visited by increasing transport cost. Once enough candidates inside the band are found,
        // further ones cannot improve their order and are unnecessary as bounded complete-goal fallbacks.
        if violation_excess == 0. {
            admissible_count += 1;
        }
        if admissible_count == band_candidate_limit {
            break;
        }
    }

    evaluated.sort_unstable_by(|left, right| {
        left.violation_excess
            .total_cmp(&right.violation_excess)
            .then_with(|| left.estimated_cost.total_cmp(&right.estimated_cost))
    });
    evaluated
}

fn retain_best_transport_candidates(candidates: &mut Vec<TailExchangeCandidate>, limit: usize) {
    if candidates.len() > limit {
        candidates.select_nth_unstable_by(limit, |left, right| left.estimated_cost.total_cmp(&right.estimated_cost));
        candidates.truncate(limit);
    }
}

fn is_route_locally_feasible(
    insertion_ctx: &InsertionContext,
    ordered_routes: &[Vec<Job>],
    candidate: &TailExchangeCandidate,
) -> bool {
    create_candidate_routes(insertion_ctx, ordered_routes, candidate).is_some()
}

fn create_candidate_routes(
    insertion_ctx: &InsertionContext,
    ordered_routes: &[Vec<Job>],
    candidate: &TailExchangeCandidate,
) -> Option<(RouteContext, RouteContext)> {
    // This screen catches route-local failures such as capacity and time windows without cloning the
    // complete solution. Constraints which depend on shared solution state are intentionally left to
    // `exchange_tails`; consequently this is a ranking heuristic, not a feasibility guarantee.
    let first_tail = &ordered_routes[candidate.first_route_idx][candidate.first_position + 1..];
    let second_tail = &ordered_routes[candidate.second_route_idx][candidate.second_position + 1..];
    let mut first_route = insertion_ctx.solution.routes.get(candidate.first_route_idx)?.deep_copy();
    let mut second_route = insertion_ctx.solution.routes.get(candidate.second_route_idx)?.deep_copy();

    if !remove_jobs_from_route(insertion_ctx, &mut first_route, first_tail)
        || !remove_jobs_from_route(insertion_ctx, &mut second_route, second_tail)
    {
        return None;
    }

    (can_insert_jobs_at_end(insertion_ctx, &mut first_route, second_tail)
        && can_insert_jobs_at_end(insertion_ctx, &mut second_route, first_tail))
    .then_some((first_route, second_route))
}

fn remove_jobs_from_route(insertion_ctx: &InsertionContext, route_ctx: &mut RouteContext, jobs: &[Job]) -> bool {
    if jobs.iter().any(|job| !route_ctx.route_mut().tour.remove(job)) {
        return false;
    }
    insertion_ctx.problem.goal.accept_route_state(route_ctx);

    true
}

fn can_insert_jobs_at_end(insertion_ctx: &InsertionContext, route_ctx: &mut RouteContext, jobs: &[Job]) -> bool {
    let result_selector = BestResultSelector::default();

    for job in jobs {
        let eval_ctx = EvaluationContext {
            goal: insertion_ctx.problem.goal.as_ref(),
            job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &result_selector,
        };
        let result = eval_job_insertion_in_route(
            insertion_ctx,
            &eval_ctx,
            route_ctx,
            InsertionPosition::Last,
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return false;
        };

        success.activities.into_iter().for_each(|(activity, index)| {
            route_ctx.route_mut().tour.insert_at(activity, index + 1);
        });
        insertion_ctx.problem.goal.accept_route_state(route_ctx);
    }

    true
}

fn get_ordered_jobs(route: &Route) -> Vec<Job> {
    let mut used = HashSet::new();

    route.tour.all_activities().filter_map(Activity::retrieve_job).filter(|job| used.insert(job.clone())).collect()
}

fn estimate_exchange_cost(
    insertion_ctx: &InsertionContext,
    first_route_idx: usize,
    first_job: &Job,
    second_route_idx: usize,
    second_job: &Job,
) -> Option<Cost> {
    let first_route = insertion_ctx.solution.routes.get(first_route_idx)?.route();
    let second_route = insertion_ctx.solution.routes.get(second_route_idx)?.route();
    let first = get_cut_activities(first_route, first_job)?;
    let second = get_cut_activities(second_route, second_job)?;
    let transport = insertion_ctx.problem.transport.as_ref();

    let old_cost = transport.cost(
        first_route,
        first.0.place.location,
        first.1.place.location,
        TravelTime::Departure(first.0.schedule.departure),
    ) + transport.cost(
        second_route,
        second.0.place.location,
        second.1.place.location,
        TravelTime::Departure(second.0.schedule.departure),
    );
    let new_cost = transport.cost(
        first_route,
        first.0.place.location,
        second.1.place.location,
        TravelTime::Departure(first.0.schedule.departure),
    ) + transport.cost(
        second_route,
        second.0.place.location,
        first.1.place.location,
        TravelTime::Departure(second.0.schedule.departure),
    );

    Some(new_cost - old_cost)
}

fn get_cut_activities<'a>(route: &'a Route, job: &Job) -> Option<(&'a Activity, &'a Activity)> {
    let cut_idx = route.tour.index_last(job)?;
    route.tour.get(cut_idx).zip(route.tour.get(cut_idx + 1))
}

fn exchange_tails(insertion_ctx: &InsertionContext, exchange: TailExchange) -> Option<InsertionContext> {
    let first_tail = insertion_ctx
        .solution
        .routes
        .get(exchange.first_route_idx)
        .map(|route_ctx| get_ordered_jobs(route_ctx.route()))?
        .into_iter()
        .skip(exchange.first_position + 1)
        .collect::<Vec<_>>();
    let second_tail = insertion_ctx
        .solution
        .routes
        .get(exchange.second_route_idx)
        .map(|route_ctx| get_ordered_jobs(route_ctx.route()))?
        .into_iter()
        .skip(exchange.second_position + 1)
        .collect::<Vec<_>>();
    let mut candidate = insertion_ctx.deep_copy();

    remove_tail(&mut candidate, exchange.first_route_idx, first_tail.as_slice())?;
    remove_tail(&mut candidate, exchange.second_route_idx, second_tail.as_slice())?;
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    insert_tail(&mut candidate, exchange.first_route_idx, second_tail)?;
    insert_tail(&mut candidate, exchange.second_route_idx, first_tail)?;
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    Some(candidate)
}

fn remove_tail(insertion_ctx: &mut InsertionContext, route_idx: usize, jobs: &[Job]) -> Option<()> {
    {
        let route_ctx = insertion_ctx.solution.routes.get_mut(route_idx)?;
        for job in jobs {
            if !route_ctx.route_mut().tour.remove(job) {
                return None;
            }
        }
        insertion_ctx.problem.goal.accept_route_state(route_ctx);
    }
    insertion_ctx.solution.required.extend(jobs.iter().cloned());

    Some(())
}

fn insert_tail(insertion_ctx: &mut InsertionContext, route_idx: usize, jobs: Vec<Job>) -> Option<()> {
    let result_selector = BestResultSelector::default();

    for job in jobs {
        if insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return None;
        }

        let eval_ctx = EvaluationContext {
            goal: insertion_ctx.problem.goal.as_ref(),
            job: &job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &result_selector,
        };
        let route_ctx = insertion_ctx.solution.routes.get(route_idx)?;
        let result = eval_job_insertion_in_route(
            insertion_ctx,
            &eval_ctx,
            route_ctx,
            InsertionPosition::Last,
            InsertionResult::make_failure(),
        );
        let InsertionResult::Success(success) = result else {
            return None;
        };

        apply_insertion_success(insertion_ctx, success);
    }

    Some(())
}
