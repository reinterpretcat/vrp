use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;

/// A recreate method which always insert fist the farthest job in empty route.
pub struct RecreateWithFarthest {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithFarthest {
    fn default() -> Self {
        Self::new(
            Box::new(AllJobSelector::default()),
            Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(FarthestResultSelector {}),
            )),
        )
    }
}

impl RecreateWithFarthest {
    /// Creates a new instance of `RecreateWithFarthest`.
    pub fn new(
        job_selector: Box<dyn JobSelector + Send + Sync>,
        job_reducer: Box<dyn JobMapReducer + Send + Sync>,
    ) -> Self {
        Self { job_selector, job_reducer }
    }
}

impl Recreate for RecreateWithFarthest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}

struct FarthestResultSelector {}

impl ResultSelector for FarthestResultSelector {
    fn select(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        match (&left, &right) {
            (InsertionResult::Success(_), InsertionResult::Failure(_)) => left,
            (InsertionResult::Failure(_), InsertionResult::Success(_)) => right,
            (InsertionResult::Success(lhs), InsertionResult::Success(rhs)) => {
                let insert_right = match (lhs.context.route.tour.has_jobs(), rhs.context.route.tour.has_jobs()) {
                    (false, false) => lhs.cost < rhs.cost,
                    (true, false) => false,
                    (false, true) => true,
                    (true, true) => lhs.cost > rhs.cost,
                };

                if insert_right {
                    right
                } else {
                    left
                }
            }
            _ => right,
        }
    }
}
