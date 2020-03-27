use crate::refinement::termination::Termination;
use crate::refinement::{Individuum, RefinementContext};
use crate::utils::Timer;

/// Stops when maximum time is passed since construction of object.
pub struct MaxTime {
    start: Timer,
    limit_in_secs: f64,
}

impl MaxTime {
    /// Creates a new instance of [`MaxTime`].
    pub fn new(limit_in_secs: f64) -> Self {
        Self { start: Timer::start(), limit_in_secs }
    }
}

impl Default for MaxTime {
    fn default() -> Self {
        Self::new(300.)
    }
}

impl Termination for MaxTime {
    fn is_termination(&self, _refinement_ctx: &mut RefinementContext, _: (&Individuum, bool)) -> bool {
        self.start.elapsed_secs_as_f64() > self.limit_in_secs
    }
}
