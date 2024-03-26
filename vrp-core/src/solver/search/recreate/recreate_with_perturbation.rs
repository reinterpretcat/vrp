use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rosomaxa::prelude::{Noise, Random};

/// A recreate method which perturbs the cost by a factor to introduce randomization.
pub struct RecreateWithPerturbation {
    recreate: ConfigurableRecreate,
}

impl RecreateWithPerturbation {
    /// Creates a new instance of `RecreateWithPerturbation`.
    pub fn new(noise: Noise, random: Random) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<AllJobSelector>::default(),
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random.clone()),
                ResultSelection::Concrete(Box::new(NoiseResultSelector::new(noise))),
                Default::default(),
            ),
        }
    }

    /// Creates a new instance of `RecreateWithPerturbation` with default values.
    pub fn new_with_defaults(random: Random) -> Self {
        Self::new(Noise::new_with_ratio(0.05, (-0.25, 0.25), random.clone()), random)
    }
}

impl Recreate for RecreateWithPerturbation {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
