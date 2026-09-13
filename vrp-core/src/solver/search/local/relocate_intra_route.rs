#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/relocate_intra_route_test.rs"]
mod relocate_intra_route_test;

use super::get_path_cost;
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use rosomaxa::prelude::{HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;

/// A cost-guided local search which relocates one job within its current route.
///
/// Removal saving provides a cheap, problem-size-independent source shortlist. The configured
/// insertion goal then ranks each source's best reinsertion using route-local copies. Only a small
/// set of predicted improvements is materialized as complete solutions, where the normal constraint
/// pipeline and complete configured objective make the authoritative decision. Locked jobs are not
/// moved and multi-activity jobs are left to sequence neighborhoods.
pub struct RelocateIntraRoute {
    source_job_threshold: usize,
    candidate_threshold: usize,
}

impl RelocateIntraRoute {
    /// Creates a new instance of `RelocateIntraRoute`.
    pub fn new(source_job_threshold: usize, candidate_threshold: usize) -> Self {
        assert!(source_job_threshold > 0);
        assert!(candidate_threshold > 0);

        Self { source_job_threshold, candidate_threshold }
    }
}

impl Default for RelocateIntraRoute {
    fn default() -> Self {
        // Removal saving is a coarse source score, so retain the same broad source set as the
        // inter-route relocation search before applying the more precise insertion estimate.
        Self::new(32, 8)
    }
}

impl LocalOperator for RelocateIntraRoute {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let is_quota_reached = || insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached());
        if is_quota_reached() {
            return None;
        }

        select_candidates(insertion_ctx, self.source_job_threshold, self.candidate_threshold)
            .into_iter()
            .take_while(|_| !is_quota_reached())
            .find_map(|candidate| {
                apply_relocation(insertion_ctx, candidate.route_idx, candidate.job).filter(|candidate| {
                    insertion_ctx.problem.goal.total_order(candidate, insertion_ctx) == Ordering::Less
                })
            })
    }
}

struct Source {
    removal_cost: Cost,
    route_idx: usize,
    insertion_idx: usize,
    job: Job,
}

struct Candidate {
    estimated_cost: InsertionCost,
    route_idx: usize,
    insertion_idx: usize,
    job: Job,
}

fn select_candidates(
    insertion_ctx: &InsertionContext,
    source_job_threshold: usize,
    candidate_threshold: usize,
) -> Vec<Candidate> {
    let is_quota_reached = || insertion_ctx.environment.quota.as_ref().is_some_and(|quota| quota.is_reached());
    let locked = &insertion_ctx.solution.locked;
    let mut sources = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter(|(_, route_ctx)| route_ctx.route().tour.job_count() > 1)
        .flat_map(|(route_idx, route_ctx)| {
            let route = route_ctx.route();

            route.tour.all_activities().enumerate().filter_map(move |(insertion_idx, activity)| {
                let job = activity.retrieve_job()?;
                if job.as_single().is_none() || locked.contains(&job) {
                    return None;
                }

                estimate_removal_cost(insertion_ctx, route, insertion_idx).map(|removal_cost| Source {
                    removal_cost,
                    route_idx,
                    insertion_idx,
                    job,
                })
            })
        })
        .collect::<Vec<_>>();

    retain_best(&mut sources, source_job_threshold, |left, right| {
        left.removal_cost
            .total_cmp(&right.removal_cost)
            .then_with(|| left.route_idx.cmp(&right.route_idx))
            .then_with(|| left.insertion_idx.cmp(&right.insertion_idx))
    });

    let mut candidates = sources
        .into_iter()
        .take_while(|_| !is_quota_reached())
        .filter_map(|source| {
            estimate_relocation_cost(insertion_ctx, source.route_idx, source.insertion_idx, &source.job).map(
                |estimated_cost| Candidate {
                    estimated_cost,
                    route_idx: source.route_idx,
                    insertion_idx: source.insertion_idx,
                    job: source.job,
                },
            )
        })
        .filter(|candidate| candidate.estimated_cost < InsertionCost::default())
        .collect::<Vec<_>>();

    retain_best(&mut candidates, candidate_threshold, |left, right| {
        left.estimated_cost
            .cmp(&right.estimated_cost)
            .then_with(|| left.route_idx.cmp(&right.route_idx))
            .then_with(|| left.insertion_idx.cmp(&right.insertion_idx))
    });

    candidates
}

fn retain_best<T>(data: &mut Vec<T>, size: usize, compare: impl Fn(&T, &T) -> Ordering + Copy) {
    if data.len() > size {
        data.select_nth_unstable_by(size, compare);
        data.truncate(size);
    }
    data.sort_unstable_by(compare);
}

fn estimate_removal_cost(insertion_ctx: &InsertionContext, route: &Route, activity_idx: usize) -> Option<Cost> {
    let previous = route.tour.get(activity_idx.checked_sub(1)?)?;
    let activity = route.tour.get(activity_idx)?;
    let next = route.tour.get(activity_idx + 1);
    let old_cost =
        get_path_cost(insertion_ctx, route, std::iter::once(previous).chain(std::iter::once(activity)).chain(next));
    let new_cost = get_path_cost(insertion_ctx, route, std::iter::once(previous).chain(next));

    Some(new_cost - old_cost)
}

fn estimate_relocation_cost(
    insertion_ctx: &InsertionContext,
    route_idx: usize,
    insertion_idx: usize,
    job: &Job,
) -> Option<InsertionCost> {
    let route_ctx = insertion_ctx.solution.routes.get(route_idx)?;
    let mut route_ctx = route_ctx.deep_copy();
    if !route_ctx.route_mut().tour.remove(job) {
        return None;
    }
    insertion_ctx.problem.goal.accept_route_state(&mut route_ctx);

    let result_selector = BestResultSelector::default();
    let eval_ctx = EvaluationContext {
        goal: insertion_ctx.problem.goal.as_ref(),
        job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &result_selector,
    };
    let original = eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        &route_ctx,
        InsertionPosition::Concrete(insertion_idx.checked_sub(1)?),
        InsertionResult::make_failure(),
    )
    .as_success()?
    .cost
    .clone();
    let best = eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        &route_ctx,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    )
    .as_success()?
    .cost
    .clone();

    Some(best - original)
}

fn apply_relocation(insertion_ctx: &InsertionContext, route_idx: usize, job: Job) -> Option<InsertionContext> {
    let mut candidate = insertion_ctx.deep_copy();
    let route_ctx = candidate.solution.routes.get_mut(route_idx)?;
    if !route_ctx.route_mut().tour.remove(&job) {
        return None;
    }
    candidate.solution.required.push(job.clone());
    candidate.problem.goal.accept_route_state(route_ctx);
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    let result_selector = BestResultSelector::default();
    let eval_ctx = EvaluationContext {
        goal: candidate.problem.goal.as_ref(),
        job: &job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &result_selector,
    };
    let route_ctx = candidate.solution.routes.get(route_idx)?;
    let InsertionResult::Success(success) = eval_job_insertion_in_route(
        &candidate,
        &eval_ctx,
        route_ctx,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    ) else {
        return None;
    };

    apply_insertion_success(&mut candidate, success);
    finalize_insertion_ctx(&mut candidate);

    Some(candidate)
}
