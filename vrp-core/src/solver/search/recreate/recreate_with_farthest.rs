use crate::construction::heuristics::InsertionContext;
use crate::construction::heuristics::*;
use crate::solver::search::recreate::Recreate;
use crate::solver::search::ConfigurableRecreate;
use crate::solver::RefinementContext;
use rosomaxa::prelude::Random;
use std::sync::Arc;

/// A recreate method which always insert first the farthest job in empty route and prefers
/// filling non-empty routes first.
pub struct RecreateWithFarthest {
    recreate: ConfigurableRecreate,
}

impl RecreateWithFarthest {
    /// Creates a new instance of `RecreateWithFarthest`.
    pub fn new(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self {
            recreate: ConfigurableRecreate::new(
                Box::<AllJobSelector>::default(),
                Box::<AllRouteSelector>::default(),
                LegSelection::Stochastic(random),
                ResultSelection::Concrete(Box::<FarthestResultSelector>::default()),
                Default::default(),
            ),
        }
    }
}

impl Recreate for RecreateWithFarthest {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreate.run(refinement_ctx, insertion_ctx)
    }
}
