#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::Acceptance;
use crate::refinement::RefinementContext;
use std::sync::Arc;

/// Greedy acceptance which accepts only better solutions.
pub struct Greedy {
    is_minimize_routes: bool,
}

impl Greedy {
    pub fn new(is_minimize_routes: bool) -> Self {
        Self { is_minimize_routes }
    }
}

impl Default for Greedy {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Acceptance for Greedy {
    fn is_accepted(&self, refinement_ctx: &RefinementContext, solution: (&InsertionContext, ObjectiveCost)) -> bool {
        match refinement_ctx.population.first() {
            Some(best) => {
                let minimize_routes_check = if self.is_minimize_routes {
                    solution.0.solution.routes.len() <= best.0.solution.routes.len()
                } else {
                    false
                };
                minimize_routes_check || solution.1.total() < best.1.total()
            }
            None => true,
        }
    }
}
