use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;
use crate::utils::Random;

/// A recreate method which perturbs the cost by a factor to introduce randomization.
pub struct RecreateWithPerturbation {
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    job_selector: Box<dyn JobSelector + Send + Sync>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
}

impl Default for RecreateWithPerturbation {
    fn default() -> Self {
        Self::new(0.33, 0.8, 1.2)
    }
}

impl RecreateWithPerturbation {
    /// Creates a new instance of `RecreateWithPerturbation`.
    pub fn new(probability: f64, min: f64, max: f64) -> Self {
        Self {
            route_selector: Box::new(AllRouteSelector::default()),
            job_selector: Box::new(AllJobSelector::default()),
            job_reducer: Box::new(PairJobMapReducer::new(Box::new(CostPerturbationResultSelector::new(
                probability,
                min,
                max,
            )))),
        }
    }
}

impl Recreate for RecreateWithPerturbation {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        InsertionHeuristic::default().process(
            self.route_selector.as_ref(),
            self.job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}

/// Selects best result.
struct CostPerturbationResultSelector {
    probability: f64,
    min: f64,
    max: f64,
}

impl CostPerturbationResultSelector {
    pub fn new(probability: f64, min: f64, max: f64) -> Self {
        Self { probability, min, max }
    }
}

impl ResultSelector for CostPerturbationResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        InsertionResult::choose_best_result(
            self.try_perturbation(left, ctx.random.as_ref()),
            self.try_perturbation(right, ctx.random.as_ref()),
        )
    }
}

impl CostPerturbationResultSelector {
    fn try_perturbation(&self, result: InsertionResult, random: &dyn Random) -> InsertionResult {
        if random.uniform_real(0., 1.) < self.probability {
            match result {
                InsertionResult::Success(success) => InsertionResult::Success(InsertionSuccess {
                    cost: success.cost * random.uniform_real(self.min, self.max),
                    job: success.job,
                    activities: success.activities,
                    context: success.context,
                }),
                _ => result,
            }
        } else {
            result
        }
    }
}
