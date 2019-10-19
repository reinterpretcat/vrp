use crate::models::common::ObjectiveCost;
use crate::models::problem::Job;
use crate::models::{Problem, Solution};
use crate::utils::Random;
use std::collections::HashSet;
use std::sync::Arc;

/// Contains information needed to perform refinement.
pub struct RefinementContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Specifies jobs which should not be affected.
    pub locked: Arc<HashSet<Arc<Job>>>,

    /// Specifies sorted collection discovered and accepted solutions with their cost.
    pub population: Vec<(Arc<Solution>, ObjectiveCost)>,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,

    /// Specifies refinement generation (or iteration).
    pub generation: usize,
}

impl RefinementContext {
    /// Returns first individuum from population.
    pub fn individuum(&self) -> Result<&(Arc<Solution>, ObjectiveCost), String> {
        self.population.first().ok_or_else(|| "Empty population".to_string())
    }
}

pub mod ruin;
