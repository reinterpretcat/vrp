#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/relocate_inter_route_test.rs"]
mod relocate_inter_route_test;

use super::create_route_pairs;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::{LocalOperator, create_environment_with_custom_quota};
use rosomaxa::prelude::{HeuristicContext, HeuristicObjective, HeuristicSolution};
use std::cmp::Ordering;

/// A granular local-search operator which relocates one job between candidate route pairs.
pub struct RelocateInterRouteBest {
    route_pairs_threshold: usize,
}

impl RelocateInterRouteBest {
    /// Creates a new instance of `RelocateInterRouteBest`.
    pub fn new(route_pairs_threshold: usize) -> Self {
        assert!(route_pairs_threshold > 0);

        Self { route_pairs_threshold }
    }
}

impl Default for RelocateInterRouteBest {
    fn default() -> Self {
        Self::new(8)
    }
}

impl LocalOperator for RelocateInterRouteBest {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        if insertion_ctx.solution.routes.len() < 2 {
            return None;
        }

        let route_pairs = create_route_pairs(insertion_ctx, self.route_pairs_threshold);
        let limit =
            refinement_ctx.statistics().speed.get_median().map(|median| ((median.max(10) as f64) * 1.5) as usize);
        let insertion_ctx = InsertionContext {
            environment: create_environment_with_custom_quota(limit, insertion_ctx.environment.as_ref()),
            ..insertion_ctx.deep_copy()
        };

        find_best_relocation(&insertion_ctx, route_pairs.as_slice())
            .map(|result| InsertionContext { environment: refinement_ctx.environment.clone(), ..result })
    }
}

fn find_best_relocation(insertion_ctx: &InsertionContext, route_pairs: &[(usize, usize)]) -> Option<InsertionContext> {
    let goal = insertion_ctx.problem.goal.as_ref();
    let quota = insertion_ctx.environment.quota.as_ref();
    let mut best: Option<InsertionContext> = None;

    for &(outer_idx, inner_idx) in route_pairs {
        for (source_idx, target_idx) in [(outer_idx, inner_idx), (inner_idx, outer_idx)] {
            let jobs = insertion_ctx.solution.routes[source_idx]
                .route()
                .tour
                .jobs()
                .filter(|job| !insertion_ctx.solution.locked.contains(*job))
                .cloned()
                .collect::<Vec<_>>();

            for job in jobs {
                if quota.is_some_and(|quota| quota.is_reached()) {
                    return best;
                }

                let Some(candidate) = relocate_job(insertion_ctx, source_idx, target_idx, &job) else {
                    continue;
                };

                if goal.total_order(&candidate, insertion_ctx) != Ordering::Less {
                    continue;
                }

                if best.as_ref().is_none_or(|best| goal.total_order(&candidate, best) == Ordering::Less) {
                    best = Some(candidate);
                }
            }
        }
    }

    best
}

fn relocate_job(
    insertion_ctx: &InsertionContext,
    source_idx: usize,
    target_idx: usize,
    job: &Job,
) -> Option<InsertionContext> {
    let mut candidate = insertion_ctx.deep_copy();
    let source_route = candidate.solution.routes.get_mut(source_idx)?;

    if !source_route.route_mut().tour.remove(job) {
        return None;
    }
    candidate.problem.goal.accept_route_state(source_route);

    let target_route = candidate.solution.routes.get(target_idx)?;
    let eval_ctx = EvaluationContext {
        goal: candidate.problem.goal.as_ref(),
        job,
        leg_selection: &LegSelection::Exhaustive,
        result_selector: &BestResultSelector::default(),
    };
    let result = eval_job_insertion_in_route(
        &candidate,
        &eval_ctx,
        target_route,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    );

    let InsertionResult::Success(success) = result else {
        return None;
    };

    apply_insertion_success(&mut candidate, success);
    candidate.solution.remove_empty_routes();
    candidate.problem.goal.accept_solution_state(&mut candidate.solution);

    Some(candidate)
}
