use crate::construction::states::InsertionContext;
use crate::models::common::ObjectiveCost;
use crate::refinement::termination::Termination;
use crate::refinement::RefinementContext;
use std::time::Instant;

/// Stops when maximum time is passed since construction of object.
pub struct MaxTime {
    start: Instant,
    limit_in_secs: f64,
}

impl MaxTime {
    pub fn new(limit_in_secs: f64) -> Self {
        Self { start: Instant::now(), limit_in_secs }
    }
}

impl Default for MaxTime {
    fn default() -> Self {
        Self::new(300.)
    }
}

impl Termination for MaxTime {
    fn is_termination(
        &mut self,
        _refinement_ctx: &RefinementContext,
        _: (&InsertionContext, ObjectiveCost, bool),
    ) -> bool {
        self.start.elapsed().as_secs_f64() > self.limit_in_secs
    }
}
