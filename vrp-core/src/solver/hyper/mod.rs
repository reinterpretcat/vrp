//! This module contains a hyper-heuristic logic.

mod dynamic_selective;
pub use self::dynamic_selective::*;

mod static_selective;
pub use self::static_selective::*;

use crate::models::Problem;
use crate::solver::population::Individual;
use crate::solver::{RefinementContext, RefinementSpeed};
use crate::utils::{Environment, Random};
use hashbrown::HashMap;
use std::ops::Deref;
use std::sync::Arc;

/// Represents a hyper heuristic functionality.
pub trait HyperHeuristic {
    /// Performs a new search in solution space using individuals provided.
    fn search(&mut self, refinement_ctx: &RefinementContext, individuals: Vec<&Individual>) -> Vec<Individual>;
}

/// A selective heuristic which uses dynamic or static selective heuristic depending on search performance.
pub struct MultiSelective {
    inner: Box<dyn HyperHeuristic + Send + Sync>,
    is_slow_search: bool,
}

impl HyperHeuristic for MultiSelective {
    fn search(&mut self, refinement_ctx: &RefinementContext, individuals: Vec<&Individual>) -> Vec<Individual> {
        self.is_slow_search = match (self.is_slow_search, &refinement_ctx.statistics.speed) {
            (false, RefinementSpeed::Slow(ratio)) => {
                refinement_ctx.environment.logger.deref()(&format!(
                    "slow refinement speed ({}), switch to static selective hyper-heuristic",
                    *ratio
                ));
                self.inner = Box::new(StaticSelective::new_with_defaults(
                    refinement_ctx.problem.clone(),
                    refinement_ctx.environment.clone(),
                ));
                true
            }
            (true, RefinementSpeed::Slow(_)) => true,
            _ => false,
        };

        self.inner.search(refinement_ctx, individuals)
    }
}

impl MultiSelective {
    /// Creates an instance of `MultiSelective` heuristic.
    pub fn new_with_defaults(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        MultiSelective {
            inner: Box::new(DynamicSelective::new_with_defaults(problem, environment)),
            is_slow_search: false,
        }
    }
}
