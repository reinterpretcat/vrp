//! Contains logic to build a feasible solution from partially ruined one.

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;

/// A trait which specifies logic to produce a new feasible solution from partial one.
pub trait Recreate {
    /// Recreates a new solution from the given.
    fn run(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod recreate_with_cheapest;
pub use self::recreate_with_cheapest::RecreateWithCheapest;

mod recreate_with_gaps;
pub use self::recreate_with_gaps::RecreateWithGaps;

mod recreate_with_blinks;
pub use self::recreate_with_blinks::RecreateWithBlinks;

mod recreate_with_regret;
pub use self::recreate_with_regret::RecreateWithRegret;

mod recreate_with_nearest_neighbor;
pub use self::recreate_with_nearest_neighbor::*;

/// Provides the way to run one of multiple recreate methods.
pub struct CompositeRecreate {
    recreates: Vec<Box<dyn Recreate>>,
    weights: Vec<usize>,
}

impl Default for CompositeRecreate {
    fn default() -> Self {
        Self::new(vec![
            (Box::new(RecreateWithCheapest::default()), 100),
            (Box::new(RecreateWithRegret::default()), 90),
            (Box::new(RecreateWithBlinks::<i32>::default()), 30),
            (Box::new(RecreateWithRegret::new(5, 8)), 20),
            (Box::new(RecreateWithGaps::default()), 10),
            (Box::new(RecreateWithNearestNeighbor::default()), 5),
        ])
    }
}

impl CompositeRecreate {
    pub fn new(recreates: Vec<(Box<dyn Recreate>, usize)>) -> Self {
        let weights = recreates.iter().map(|(_, weight)| *weight).collect();
        let recreates = recreates.into_iter().map(|(recreate, _)| recreate).collect();
        Self { recreates, weights }
    }
}

impl Recreate for CompositeRecreate {
    fn run(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.random.weighted(self.weights.as_slice());
        self.recreates.get(index).unwrap().run(refinement_ctx, insertion_ctx)
    }
}
