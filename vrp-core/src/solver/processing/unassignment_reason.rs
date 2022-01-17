use super::*;
use crate::construction::heuristics::*;
use rosomaxa::utils::{parallel_into_collect, CollectGroupBy};

/// Tries to improve job unassignment reason.
#[derive(Default)]
pub struct UnassignmentReason {}

impl HeuristicSolutionProcessing for UnassignmentReason {
    type Solution = InsertionContext;

    fn process(&self, solution: Self::Solution) -> Self::Solution {
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
            let mut unassigned = insertion_ctx
                .solution
                .routes
                .iter()
                .map(|route_ctx| {
                    evaluate_job_insertion_in_route(
                        &insertion_ctx,
                        &eval_ctx,
                        route_ctx,
                        InsertionPosition::Any,
                        InsertionResult::make_failure(),
                    )
                })
                .filter_map(|result| match &result {
                    InsertionResult::Failure(failure) if failure.constraint > 0 => Some(failure.constraint),
                    _ => None,
                })
                .collect_group_by_key(|code| *code)
                .into_iter()
                .map(|code_stat| (code_stat.0, code_stat.1.len()))
                .collect::<Vec<_>>();

            unassigned.sort_by(|(_, a), (_, b)| b.cmp(a));
            let frequent_code = unassigned.first().map(|(code, _)| *code).unwrap_or(code);

            (job, frequent_code)
        });

        insertion_ctx.solution.unassigned.extend(unassigned.into_iter());

        insertion_ctx
    }
}
