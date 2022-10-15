use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rosomaxa::prelude::{Noise, Random};
use std::sync::Arc;

/// A recreate method which perturbs the cost by a factor to introduce randomization.
pub struct RecreateWithPerturbation {
    recreate: ConfigurableRecreate,
}

impl RecreateWithPerturbation {
    /// Creates a new instance of `RecreateWithPerturbation`.
    pub fn new(noise: Noise, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::new(AllJobSelector::default()),
                Box::new(AllRouteSelector::default()),
                Box::new(VariableLegSelector::new(random)),
                Box::new(NoiseResultSelector::new(noise)),
                Default::default(),
            ),
        }
    }

    /// Creates a new instance of `RecreateWithPerturbation` with default values.
    pub fn new_with_defaults(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self::new(Noise::new(0.05, (0.75, 1.25), random.clone()), random)
    }
}

impl Recreate for RecreateWithPerturbation {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
