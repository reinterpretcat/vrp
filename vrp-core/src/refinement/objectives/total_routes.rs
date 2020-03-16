use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCostType};
use crate::refinement::RefinementContext;

/// An objective function which counts total amount of routes.
pub struct TotalRoutes {
    goal: Option<(usize, bool)>,
}

impl Default for TotalRoutes {
    fn default() -> Self {
        Self { goal: None }
    }
}

impl TotalRoutes {
    pub fn new(desired_routes: usize, is_minimization: bool) -> Self {
        Self { goal: Some((desired_routes, is_minimization)) }
    }
}

impl Objective for TotalRoutes {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.routes.len() as Cost))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> Option<bool> {
        let total_routes = insertion_ctx.solution.routes.len();

        self.goal
            .map(
                |(desired_routes, is_minimization)| {
                    if is_minimization {
                        total_routes <= desired_routes
                    } else {
                        total_routes >= desired_routes
                    }
                },
            )
            .or(None)
    }
}
