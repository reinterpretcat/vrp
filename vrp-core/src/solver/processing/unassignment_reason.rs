#[cfg(test)]
#[path = "../../../tests/unit/solver/processing/unassignment_reason_test.rs"]
mod unassignment_reason_test;

use super::*;
use crate::construction::heuristics::*;
use rosomaxa::utils::{parallel_into_collect, CollectGroupBy};

/// Tries to improve job unassignment reason.
#[derive(Default)]
pub struct UnassignmentReason {}

impl HeuristicSolutionProcessing for UnassignmentReason {
    type Solution = InsertionContext;

    fn post_process(&self, solution: Self::Solution) -> Self::Solution {
        let mut insertion_ctx = solution;

        let unassigned = insertion_ctx.solution.unassigned.drain().collect::<Vec<_>>();
        let leg_selector = VariableLegSelector::new(insertion_ctx.environment.random.clone());
        let result_selector = BestResultSelector::default();

        let unassigned = parallel_into_collect(unassigned, |(job, code)| {
            let eval_ctx = EvaluationContext {
                goal: &insertion_ctx.problem.goal,
                job: &job,
                leg_selector: &leg_selector,
                result_selector: &result_selector,
            };
            let details = insertion_ctx
                .solution
                .routes
                .iter()
                .filter_map(|route_ctx| {
                    (0..route_ctx.route.tour.legs().count())
                        .map(|leg_idx| {
                            evaluate_job_insertion_in_route(
                                &insertion_ctx,
                                &eval_ctx,
                                route_ctx,
                                InsertionPosition::Concrete(leg_idx),
                                InsertionResult::make_failure(),
                            )
                        })
                        .filter_map(|result| match result {
                            InsertionResult::Failure(failure) => Some(failure),
                            _ => None,
                        })
                        .collect_group_by_key(|code| code.constraint)
                        .into_iter()
                        // NOTE: pick only the most frequent reason
                        .max_by(|(_, a), (_, b)| a.len().cmp(&b.len()))
                        .map(|(code, _)| (route_ctx.route.actor.clone(), code))
                })
                .collect::<Vec<_>>();

            let code = if details.is_empty() { code } else { UnassignmentInfo::Detailed(details) };

            (job, code)
        });

        insertion_ctx.solution.unassigned.extend(unassigned.into_iter());

        insertion_ctx
    }
}
