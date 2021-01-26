//! This module contains a hyper-heuristic logic.

mod static_selective;
pub use self::static_selective::*;

use crate::solver::population::Individual;
use crate::solver::RefinementContext;
use crate::utils::Random;
use hashbrown::HashMap;

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic {
    /// Performs a new search in solution space using individuals provided.
    fn search(&mut self, refinement_ctx: &RefinementContext, individuals: Vec<&Individual>) -> Vec<Individual>;
}
