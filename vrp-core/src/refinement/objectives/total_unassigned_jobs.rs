use crate::construction::states::InsertionContext;
use crate::models::common::Cost;
use crate::refinement::objectives::{MeasurableObjectiveCost, Objective, ObjectiveCostType};
use crate::refinement::RefinementContext;

/// An objective function which counts total amount of unassigned jobs.
pub struct TotalUnassignedJobs {
    goal: Option<usize>,
}

impl Default for TotalUnassignedJobs {
    fn default() -> Self {
        Self { goal: None }
    }
}

impl TotalUnassignedJobs {
    pub fn new(desired_unassigned: usize) -> Self {
        Self { goal: Some(desired_unassigned) }
    }
}

impl Objective for TotalUnassignedJobs {
    fn estimate_cost(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> ObjectiveCostType {
        Box::new(MeasurableObjectiveCost::new(insertion_ctx.solution.unassigned.len() as Cost))
    }

    fn is_goal_satisfied(&self, _: &mut RefinementContext, insertion_ctx: &InsertionContext) -> Option<bool> {
        self.goal.map(|desired_unassigned| insertion_ctx.solution.unassigned.len() <= desired_unassigned).or(None)
    }
}
