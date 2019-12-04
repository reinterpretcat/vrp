#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::acceptance::Acceptance;
use crate::refinement::RefinementContext;
use std::cmp::Ordering;

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
        match refinement_ctx.population.all().next() {
            Some(best) => {
                let unassigned_cmp = solution.0.solution.unassigned.len().cmp(&best.0.solution.unassigned.len());
                let route_cmp = solution.0.solution.routes.len().cmp(&best.0.solution.routes.len());

                match (unassigned_cmp, route_cmp, self.is_minimize_routes) {
                    (Ordering::Less, _, _) => true,
                    (Ordering::Greater, _, _) => false,
                    (_, Ordering::Less, true) => true,
                    (_, Ordering::Greater, true) => false,
                    _ => solution.1.total() < best.1.total(),
                }
            }
            None => true,
        }
    }
}
