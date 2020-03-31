use super::*;

/// An objective function which counts total amount of routes.
pub struct TotalRoutes {
    route_goal: Option<(f64, bool)>,
    variation_goal: Option<VariationCoefficient>,
}

impl Default for TotalRoutes {
    fn default() -> Self {
        Self { route_goal: None, variation_goal: None }
    }
}

impl TotalRoutes {
    pub fn new(route_goal: Option<usize>, variation_goal: Option<(usize, f64)>, is_minimization: bool) -> Self {
        Self {
            route_goal: route_goal.map(|routes| (routes as f64, is_minimization)),
            variation_goal: variation_goal
                .map(|(sample, threshold)| VariationCoefficient::new(sample, threshold, "routes_vc")),
        }
    }
}

impl Objective for TotalRoutes {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.routes.len() as Cost))
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
