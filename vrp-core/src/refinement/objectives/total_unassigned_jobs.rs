use super::*;

/// An objective function which counts total amount of unassigned jobs.
pub struct TotalUnassignedJobs {
    unassigned_goal: Option<(f64, bool)>,
    variation_goal: Option<VariationCoefficient>,
}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self { unassigned_goal: None, variation_goal: None }
    }
}

impl TotalUnassignedJobs {
    pub fn new(desired_unassigned: Option<usize>, variation_goal: Option<(usize, f64)>) -> Self {
        Self {
            unassigned_goal: desired_unassigned.map(|unassigned| (unassigned as f64, true)),
            variation_goal: variation_goal
                .map(|(sample, threshold)| VariationCoefficient::new(sample, threshold, "unassigned_vc")),
        }
    }
}

impl Objective for TotalUnassignedJobs {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.unassigned.len() as Cost))
    }

    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool> {
        let actual_unassigned = insertion_ctx.solution.unassigned.len() as f64;

        check_value_variation_goals(refinement_ctx, actual_unassigned, &self.unassigned_goal, &self.variation_goal)
    }
}
