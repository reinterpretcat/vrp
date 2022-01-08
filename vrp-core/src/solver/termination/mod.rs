//! The termination module contains logic which defines termination criteria for metaheuristic,
//! e.g. when to stop evolution in evolutionary algorithms.

use crate::solver::RefinementContext;
use rosomaxa::utils::compare_floats;

/// A trait which specifies criteria when metaheuristic should stop searching for improved solution.
pub trait Termination {
    /// Returns true if termination condition is met.
    fn is_termination(&self, refinement_ctx: &mut RefinementContext) -> bool;

    /// Returns a relative estimation till termination. Value is in the `[0, 1]` range.
    fn estimate(&self, refinement_ctx: &RefinementContext) -> f64;
}

mod min_variation;
pub use self::min_variation::MinVariation;

mod max_generation;
pub use self::max_generation::MaxGeneration;

mod max_time;
pub use self::max_time::MaxTime;

/// A trait which encapsulates multiple termination criteria.
pub struct CompositeTermination {
    terminations: Vec<Box<dyn Termination + Send + Sync>>,
}

impl CompositeTermination {
    /// Creates a new instance of `CompositeTermination`.
    pub fn new(terminations: Vec<Box<dyn Termination + Send + Sync>>) -> Self {
        Self { terminations }
    }
}

impl Termination for CompositeTermination {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext) -> bool {
        self.terminations.iter().any(|t| t.is_termination(refinement_ctx))
    }

    fn estimate(&self, refinement_ctx: &RefinementContext) -> f64 {
        self.terminations.iter().map(|t| t.estimate(refinement_ctx)).max_by(|a, b| compare_floats(*a, *b)).unwrap_or(0.)
    }
}
