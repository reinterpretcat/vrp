#[cfg(test)]
#[path = "../../../tests/unit/refinement/acceptance/greedy_test.rs"]
mod greedy_test;

use crate::refinement::acceptance::Acceptance;
use crate::refinement::{Individuum, RefinementContext};
use crate::utils::compare_floats;
use std::cmp::Ordering;

/// Greedy acceptance which accepts only better solutions.
pub struct Greedy {
    is_minimize_routes: bool,
}

impl Greedy {
    /// Creates a new `Greedy`. `is_minimize_routes` specifies whether solution with less
    /// routes should be preferred over lower cost.
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
    fn is_accepted(&self, refinement_ctx: &mut RefinementContext, solution: &Individuum) -> bool {
        let less_cost =
            refinement_ctx.population.all().min_by(|(_, a, _), (_, b, _)| compare_floats(a.total(), b.total()));
        let less_routes = refinement_ctx
            .population
            .all()
            .min_by(|(a, _, _), (b, _, _)| a.solution.routes.len().cmp(&b.solution.routes.len()));

        match (less_cost, less_routes) {
            (Some(less_cost), Some(less_routes)) => {
                let known_unassigned =
                    less_cost.0.solution.unassigned.len().min(less_routes.0.solution.unassigned.len());

                let unassigned_cmp = solution.0.solution.unassigned.len().cmp(&known_unassigned);
                let route_cmp = solution.0.solution.routes.len().cmp(&less_routes.0.solution.routes.len());

                match (unassigned_cmp, route_cmp, self.is_minimize_routes) {
                    (Ordering::Less, _, _) => true,
                    (Ordering::Greater, _, _) => false,
                    (_, Ordering::Less, true) => true,
                    (_, Ordering::Equal, true) => solution.1.total() < less_routes.1.total(),
                    _ => solution.1.total() < less_cost.1.total(),
                }
            }
            _ => true,
        }
    }
}
