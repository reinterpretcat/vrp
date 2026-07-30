#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_two_opt_star_test.rs"]
mod exchange_two_opt_star_test;

use crate::construction::heuristics::*;
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{Job, TravelTime};
use crate::models::solution::{Activity, Route};
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use rosomaxa::prelude::{HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;
use std::collections::HashSet;

// Route-only feasibility checks are much cheaper than complete solution copies, but still scale with
// tail length. Keep this as a small fixed implementation budget rather than another runtime knob.
const SCREEN_CANDIDATE_THRESHOLD: usize = 8;

/// A granular 2-opt* operator which exchanges ordered tails between nearby routes.
///
/// Candidate cut edges are built from the problem's nearest-job index and ranked using the transport
/// delta of reconnecting the two prefixes to the opposite tails. The best few candidates are screened
/// using copies of only their two routes. Only the first route-locally feasible candidate is
/// materialized: the complete solution is copied once, both tails are removed, and their jobs are
/// inserted at the ends of the opposite prefixes through the normal constraint pipeline.
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

        let exchange = select_tail_exchange(insertion_ctx, self.neighbor_threshold, self.max_tail_jobs)?;
        let candidate = exchange_tails(insertion_ctx, exchange)?;

        (insertion_ctx.problem.goal.total_order(&candidate, insertion_ctx) == Ordering::Less).then_some(candidate)
    }
}

struct TailExchange {
    first_route_idx: usize,
    second_route_idx: usize,
    first_tail: Vec<Job>,
    second_tail: Vec<Job>,
}

struct JobPosition {
    route_idx: usize,
    position: usize,
}

struct TailExchangeCandidate {
    first_route_idx: usize,
    first_position: usize,
    second_route_idx: usize,
    second_position: usize,
    estimated_cost: Cost,
}

fn select_tail_exchange(
    insertion_ctx: &InsertionContext,
    neighbor_threshold: usize,
    max_tail_jobs: usize,
) -> Option<TailExchange> {
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
                .find(|(_, position)| {
                    if position.route_idx == first_route_idx {
                        return false;
                    }

                    let second_jobs = &ordered_routes[position.route_idx];
                    let second_tail = &second_jobs[position.position + 1..];
                    !second_tail.is_empty()
                        && second_tail.len() <= max_tail_jobs
                        && !second_tail.iter().any(|job| locked.contains(job))
                });
            let Some((second_job, second_position)) = second else {
                continue;
            };

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
            });
        }
    }

    candidates.sort_unstable_by(|left, right| left.estimated_cost.total_cmp(&right.estimated_cost));
    candidates
        .into_iter()
        .take(SCREEN_CANDIDATE_THRESHOLD)
        .find(|candidate| is_route_locally_feasible(insertion_ctx, &ordered_routes, candidate))
        .map(|candidate| TailExchange {
            first_route_idx: candidate.first_route_idx,
            second_route_idx: candidate.second_route_idx,
            first_tail: ordered_routes[candidate.first_route_idx][candidate.first_position + 1..].to_vec(),
            second_tail: ordered_routes[candidate.second_route_idx][candidate.second_position + 1..].to_vec(),
        })
}

fn is_route_locally_feasible(
    insertion_ctx: &InsertionContext,
    ordered_routes: &[Vec<Job>],
    candidate: &TailExchangeCandidate,
) -> bool {
    // This screen catches route-local failures such as capacity and time windows without cloning the
    // complete solution. Constraints which depend on shared solution state are intentionally left to
    // `exchange_tails`; consequently this is a ranking heuristic, not a feasibility guarantee.
    let first_tail = &ordered_routes[candidate.first_route_idx][candidate.first_position + 1..];
    let second_tail = &ordered_routes[candidate.second_route_idx][candidate.second_position + 1..];
    let Some(mut first_route) =
        insertion_ctx.solution.routes.get(candidate.first_route_idx).map(|route| route.deep_copy())
    else {
        return false;
    };
    let Some(mut second_route) =
        insertion_ctx.solution.routes.get(candidate.second_route_idx).map(|route| route.deep_copy())
    else {
        return false;
    };

    if !remove_jobs_from_route(insertion_ctx, &mut first_route, first_tail)
        || !remove_jobs_from_route(insertion_ctx, &mut second_route, second_tail)
    {
        return false;
    }

    can_insert_jobs_at_end(insertion_ctx, &mut first_route, second_tail)
        && can_insert_jobs_at_end(insertion_ctx, &mut second_route, first_tail)
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
    let mut candidate = insertion_ctx.deep_copy();

    remove_tail(&mut candidate, exchange.first_route_idx, exchange.first_tail.as_slice())?;
    remove_tail(&mut candidate, exchange.second_route_idx, exchange.second_tail.as_slice())?;
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    insert_tail(&mut candidate, exchange.first_route_idx, exchange.second_tail)?;
    insert_tail(&mut candidate, exchange.second_route_idx, exchange.first_tail)?;
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
