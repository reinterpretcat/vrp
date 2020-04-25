use crate::solver::termination::Termination;
use crate::solver::RefinementContext;
use crate::utils::Timer;

/// Stops when max time elapsed.
pub struct MaxTime {
    start: Timer,
    limit_in_secs: f64,
}

impl MaxTime {
    /// Creates a new instance of [`QuotaReached`].
    pub fn new(limit_in_secs: f64) -> Self {
        Self { start: Timer::start(), limit_in_secs }
    }
}

impl Termination for MaxTime {
    fn is_termination(&self, _: &mut RefinementContext) -> bool {
        self.start.elapsed_secs_as_f64() > self.limit_in_secs
    }
}
