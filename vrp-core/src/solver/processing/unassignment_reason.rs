use super::*;
use crate::construction::heuristics::*;
use crate::utils::{parallel_into_collect, CollectGroupBy};

/// Tries to improve job unassignment reason.
pub struct UnassignmentReason {}

impl Default for UnassignmentReason {
    fn default() -> Self {
        Self {}
    }
}

impl Processing for UnassignmentReason {
    fn pre_process(&self, problem: Arc<Problem>, _environment: Arc<Environment>) -> Arc<Problem> {
        problem
    }

    fn post_process(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        let unassigned = insertion_ctx.solution.unassigned.drain().collect::<Vec<_>>();
        let result_selector = BestResultSelector::default();

        let unassigned = parallel_into_collect(unassigned, |(job, code)| {
            let mut unassigned = insertion_ctx
                .solution
                .routes
                .iter()
                .map(|route_ctx| {
                    evaluate_job_insertion_in_route(
                        &insertion_ctx,
                        route_ctx,
                        &job,
                        InsertionPosition::Any,
                        InsertionResult::make_failure(),
                        &result_selector,
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
