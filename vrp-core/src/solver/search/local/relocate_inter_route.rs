#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/relocate_inter_route_test.rs"]
mod relocate_inter_route_test;

use crate::construction::heuristics::*;
use crate::models::common::{Cost, Profile, Timestamp};
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::{LocalOperator, get_route_jobs};
use rosomaxa::prelude::{HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// A cost-guided local-search operator which relocates one job to a nearby route.
///
/// The operator uses a two-stage search to avoid copying the complete solution for every possible
/// source job:
///
/// 1. It maps jobs to their current routes and uses the problem's nearest-job index to find nearby
///    target routes.
/// 2. It keeps at most `source_job_threshold` source jobs, ordered by cross-route proximity. These
///    candidates are ranked using insertion-cost deltas computed with route-local copies only.
/// 3. It copies the complete solution once for the selected source job, removes the job, refreshes
///    solution state, and evaluates every insertion position in at most `target_route_threshold`
///    nearby routes through the normal constraint pipeline.
///
/// The move is returned only when it is a strict lexicographic improvement. Therefore, the first
/// stage is only a bounded ranking heuristic; feasibility and the final objective comparison are
/// always decided using the materialized solution.
pub struct RelocateInterRoute {
    source_job_threshold: usize,
    target_route_threshold: usize,
}

impl RelocateInterRoute {
    /// Creates a new instance of `RelocateInterRoute`.
    ///
    /// `source_job_threshold` limits how many granular source candidates receive route-local delta
    /// evaluation. `target_route_threshold` limits how many nearby target routes receive exact
    /// insertion-position evaluation for the selected job.
    pub fn new(source_job_threshold: usize, target_route_threshold: usize) -> Self {
        assert!(source_job_threshold > 0);
        assert!(target_route_threshold > 0);

        Self { source_job_threshold, target_route_threshold }
    }
}

impl Default for RelocateInterRoute {
    fn default() -> Self {
        Self::new(32, 8)
    }
}

impl LocalOperator for RelocateInterRoute {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if insertion_ctx.solution.routes.len() < 2 {
            return None;
        }

        let relocation = select_relocation(insertion_ctx, self.source_job_threshold, self.target_route_threshold)?;
        let candidate = relocate_job(insertion_ctx, relocation)?;

        (insertion_ctx.problem.goal.total_order(&candidate, insertion_ctx) == Ordering::Less).then_some(candidate)
    }
}

struct Relocation {
    source_idx: usize,
    job: Job,
    target_indices: Vec<usize>,
}

fn select_relocation(
    insertion_ctx: &InsertionContext,
    source_job_threshold: usize,
    target_route_threshold: usize,
) -> Option<Relocation> {
    let route_jobs = get_route_jobs(&insertion_ctx.solution);
    let mut source_candidates = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .flat_map(|(source_idx, route_ctx)| {
            let profile = route_ctx.route().actor.vehicle.profile.clone();
            let route_jobs = &route_jobs;

            route_ctx
                .route()
                .tour
                .jobs()
                .filter(|job| !insertion_ctx.solution.locked.contains(*job))
                .cloned()
                .filter_map(move |job| {
                    let (target_indices, neighbor_cost) = get_target_indices(
                        insertion_ctx,
                        route_jobs,
                        &profile,
                        source_idx,
                        &job,
                        target_route_threshold,
                    )?;

                    Some((neighbor_cost, Relocation { source_idx, job, target_indices }))
                })
        })
        .collect::<Vec<_>>();

    source_candidates.sort_unstable_by(|left, right| left.0.total_cmp(&right.0));
    source_candidates.truncate(source_job_threshold);

    source_candidates
        .into_iter()
        .map(|(_, relocation)| {
            let estimated_cost = relocation.target_indices.first().and_then(|&target_idx| {
                estimate_relocation_cost(insertion_ctx, relocation.source_idx, target_idx, &relocation.job)
            });
            let estimated_cost = estimated_cost.unwrap_or_else(|| InsertionCost::max_value().clone());

            (estimated_cost, relocation)
        })
        .min_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, relocation)| relocation)
}

fn get_target_indices(
    insertion_ctx: &InsertionContext,
    route_jobs: &HashMap<Job, usize>,
    profile: &Profile,
    source_idx: usize,
    job: &Job,
    target_route_threshold: usize,
) -> Option<(Vec<usize>, Cost)> {
    let mut used = HashSet::new();
    let mut neighbor_cost = None;

    let target_indices = insertion_ctx
        .problem
        .jobs
        .neighbors(profile, job, Timestamp::default())
        .filter_map(|(neighbor, cost)| route_jobs.get(neighbor).copied().map(|target_idx| (target_idx, cost)))
        .filter(|(target_idx, _)| *target_idx != source_idx && used.insert(*target_idx))
        .inspect(|(_, cost)| {
            neighbor_cost.get_or_insert(*cost);
        })
        .map(|(target_idx, _)| target_idx)
        .take(target_route_threshold)
        .collect::<Vec<_>>();

    neighbor_cost.map(|cost| (target_indices, cost))
}

fn estimate_relocation_cost(
    insertion_ctx: &InsertionContext,
    source_idx: usize,
    target_idx: usize,
    job: &Job,
) -> Option<InsertionCost> {
    // This route-local delta is used only to rank the bounded source shortlist. The selected move is
    // subsequently rebuilt on a full solution copy, where shared state and all constraints are exact.
    let source_route = insertion_ctx.solution.routes.get(source_idx)?;
    let insertion_idx = source_route.route().tour.index(job)?;
    let mut source_route = source_route.deep_copy();
    source_route.route_mut().tour.remove(job);
    insertion_ctx.problem.goal.accept_route_state(&mut source_route);

    let result_selector = BestResultSelector::default();
    let eval_ctx = EvaluationContext {
        goal: insertion_ctx.problem.goal.as_ref(),
        job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &result_selector,
    };
    let removal_cost = eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        &source_route,
        InsertionPosition::Concrete(insertion_idx - 1),
        InsertionResult::make_failure(),
    )
    .as_success()
    .map(|success| success.cost.clone())
    .unwrap_or_default();

    let target_route = insertion_ctx.solution.routes.get(target_idx)?;
    let insertion_cost = eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        target_route,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    )
    .as_success()
    .map(|success| success.cost.clone())?;

    Some(&insertion_cost - removal_cost)
}

fn relocate_job(insertion_ctx: &InsertionContext, relocation: Relocation) -> Option<InsertionContext> {
    // Full solution materialization is deliberately delayed until a single source job is selected.
    let mut candidate = insertion_ctx.deep_copy();
    let source_route = candidate.solution.routes.get_mut(relocation.source_idx)?;

    if !source_route.route_mut().tour.remove(&relocation.job) {
        return None;
    }
    candidate.solution.required.push(relocation.job.clone());
    candidate.problem.goal.accept_route_state(source_route);
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    let result_selector = BestResultSelector::default();
    let eval_ctx = EvaluationContext {
        goal: candidate.problem.goal.as_ref(),
        job: &relocation.job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &result_selector,
    };
    let result = relocation.target_indices.into_iter().fold(InsertionResult::make_failure(), |best, target_idx| {
        if candidate.environment.quota.as_ref().is_some_and(|quota| quota.is_reached()) {
            return best;
        }

        match candidate.solution.routes.get(target_idx) {
            Some(target_route) => {
                eval_job_insertion_in_route(&candidate, &eval_ctx, target_route, InsertionPosition::Any, best)
            }
            None => best,
        }
    });

    let InsertionResult::Success(success) = result else {
        return None;
    };

    apply_insertion_success(&mut candidate, success);
    candidate.solution.remove_empty_routes();
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    Some(candidate)
}
