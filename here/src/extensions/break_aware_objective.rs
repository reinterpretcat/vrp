use core::construction::states::InsertionContext;
use core::models::common::{Cost, ObjectiveCost};
use core::refinement::objectives::{Objective, PenalizeUnassigned};

pub struct BreakAwareObjective {
    penalty: Cost,
    inner_objective: PenalizeUnassigned,
}

impl BreakAwareObjective {
    pub fn new(penalty: Cost) -> Self {
        Self { penalty, inner_objective: PenalizeUnassigned::new(penalty) }
    }
}

impl Default for BreakAwareObjective {
    fn default() -> Self {
        Self::new(1E6)
    }
}

impl Objective for BreakAwareObjective {
    fn estimate(&self, insertion_ctx: &InsertionContext) -> ObjectiveCost {
        let cost = self.inner_objective.estimate(insertion_ctx);

        ObjectiveCost::new(cost.actual, cost.penalty - get_late_breaks_count(insertion_ctx) as f64 * self.penalty)
    }
}

fn get_late_breaks_count(insertion_ctx: &InsertionContext) -> usize {
    unimplemented!()
}
