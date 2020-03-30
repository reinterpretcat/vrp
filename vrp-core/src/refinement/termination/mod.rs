//! Metaheuristic termination logic.

use crate::refinement::{Individuum, RefinementContext};

/// A trait which specifies criteria when metaheuristic should stop searching for improved solution.
pub trait Termination {
    /// Returns true if termination condition is met.
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, solution: (&Individuum, bool)) -> bool;
}

mod goal_satisfaction;
pub use self::goal_satisfaction::GoalSatisfaction;

mod max_generation;
pub use self::max_generation::MaxGeneration;

mod quota_reached;
pub use self::quota_reached::QuotaReached;

/// A trait which encapsulates multiple termination criteria.
pub struct CompositeTermination {
    terminations: Vec<Box<dyn Termination>>,
}

impl CompositeTermination {
    /// Creates a new instance of [`CompositeTermination`].
    pub fn new(terminations: Vec<Box<dyn Termination>>) -> Self {
        Self { terminations }
    }
}

impl Default for CompositeTermination {
    fn default() -> Self {
        Self::new(vec![Box::new(MaxGeneration::default())])
    }
}

impl Termination for CompositeTermination {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, solution: (&Individuum, bool)) -> bool {
        self.terminations.iter().any(|t| t.is_termination(refinement_ctx, solution))
    }
}
