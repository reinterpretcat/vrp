use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::mutation::ConfigurableRecreate;
use crate::solver::RefinementContext;
use crate::utils::Random;
use std::sync::Arc;

/// A recreate method which always insert first the farthest job in empty route and prefers
/// filling non-empty routes first.
pub struct RecreateWithFarthest {
    recreate: ConfigurableRecreate,
}

impl RecreateWithFarthest {
    /// Creates a new instance of `RecreateWithFarthest`.
    pub fn new(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(AllJobSelector::default()),
                Box::new(AllRouteSelector::default()),
                Box::new(VariableLegSelector::new(random)),
                Box::new(FarthestResultSelector {}),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithFarthest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}

struct FarthestResultSelector {}

impl ResultSelector for FarthestResultSelector {
    fn select_insertion(&self, _: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
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
