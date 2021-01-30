//! The recreate module contains logic to build a feasible solution from partially ruined.

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use std::sync::Arc;

/// A trait which specifies logic to produce a new feasible solution from partial one.
pub trait Recreate {
    /// Recreates a new solution from the given.
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod recreate_with_blinks;
pub use self::recreate_with_blinks::RecreateWithBlinks;

mod recreate_with_cheapest;
pub use self::recreate_with_cheapest::RecreateWithCheapest;

mod recreate_with_farthest;
pub use self::recreate_with_farthest::RecreateWithFarthest;

mod recreate_with_gaps;
pub use self::recreate_with_gaps::RecreateWithGaps;

mod recreate_with_nearest_neighbor;
pub use self::recreate_with_nearest_neighbor::RecreateWithNearestNeighbor;

mod recreate_with_perturbation;
pub use self::recreate_with_perturbation::RecreateWithPerturbation;

mod recreate_with_skip_best;
pub use self::recreate_with_skip_best::RecreateWithSkipBest;

mod recreate_with_regret;
pub use self::recreate_with_regret::RecreateWithRegret;

/// Provides the way to run one of multiple recreate methods with different probability.
pub struct CompositeRecreate {
    recreates: Vec<Arc<dyn Recreate + Send + Sync>>,
    weights: Vec<usize>,
}

impl CompositeRecreate {
    /// Creates a new instance of `CompositeRecreate` using list of recreate strategies.
    pub fn new(recreates: Vec<(Arc<dyn Recreate + Send + Sync>, usize)>) -> Self {
        let weights = recreates.iter().map(|(_, weight)| *weight).collect();
        let recreates = recreates.into_iter().map(|(recreate, _)| recreate).collect();
        Self { recreates, weights }
    }
}

impl Recreate for CompositeRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        self.recreates.get(index).unwrap().run(refinement_ctx, insertion_ctx)
    }
}
