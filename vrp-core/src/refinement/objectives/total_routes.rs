use super::*;

/// An objective function which counts total amount of routes.
pub struct TotalRoutes {
    route_goal: Option<(f64, bool)>,
    variation_goal: Option<VariationCoefficient>,
    is_minimization: bool,
}

impl Default for TotalRoutes {
    fn default() -> Self {
        Self { route_goal: None, variation_goal: None, is_minimization: true }
    }
}

impl TotalRoutes {
    pub fn new_minimized(route_goal: Option<usize>, variation_goal: Option<(usize, f64)>) -> Self {
        Self {
            route_goal: route_goal.map(|routes| (routes as f64, true)),
            variation_goal: variation_goal
                .map(|(sample, threshold)| VariationCoefficient::new(sample, threshold, "routes_vc")),
            is_minimization: true,
        }
    }

    pub fn new_maximized() -> Self {
        Self { route_goal: None, variation_goal: None, is_minimization: false }
    }
}

impl Objective for TotalRoutes {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        let cost = if self.is_minimization { 1. } else { -1. } * insertion_ctx.solution.routes.len() as Cost;
        Box::new(MeasurableObjectiveCost::new(cost))
    }

    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool> {
        let actual_routes = insertion_ctx.solution.routes.len() as f64;

        check_value_variation_goals(refinement_ctx, actual_routes, &self.route_goal, &self.variation_goal)
    }
}
