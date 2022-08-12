#[cfg(test)]
#[path = "../../../tests/unit/solver/processing/unassigned_reason_test.rs"]
mod unassigned_reason_test;

use super::*;
use crate::construction::heuristics::*;
use rosomaxa::utils::parallel_into_collect;

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
                constraint: &insertion_ctx.problem.constraint,
                job: &job,
                leg_selector: &leg_selector,
                result_selector: &result_selector,
            };
            let details = insertion_ctx
                .solution
                .routes
                .iter()
                .map(|route_ctx| {
                    (
                        route_ctx.route.actor.clone(),
                        evaluate_job_insertion_in_route(
                            &insertion_ctx,
                            &eval_ctx,
                            route_ctx,
                            InsertionPosition::Any,
                            InsertionResult::make_failure(),
                        ),
                    )
                })
                .filter_map(|(actor, result)| match &result {
                    InsertionResult::Failure(failure) => Some((actor, failure.constraint)),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let code = if details.is_empty() { code } else { UnassignedCode::Detailed(details) };

            (job, code)
        });

        insertion_ctx.solution.unassigned.extend(unassigned.into_iter());

        insertion_ctx
    }
}
