//! Contains logic to build a feasible solution from partially ruined one.

use crate::construction::states::InsertionContext;
use crate::refinement::RefinementContext;

/// A trait which specifies logic to produce a new feasible solution from partial one.
pub trait Recreate {
    /// Recreates a new solution from the given.
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
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
            (Box::new(RecreateWithGaps::default()), 10),
            (Box::new(RecreateWithNearestNeighbor::default()), 5),
        ])
    }
}

impl CompositeRecreate {
    pub fn new(recreates: Vec<(Box<dyn Recreate>, usize)>) -> Self {
        let mut recreates = recreates;
        recreates.sort_by(|(_, a), (_, b)| b.cmp(&a));

        let weights = recreates.iter().map(|(_, weight)| *weight).collect();
        Self { recreates: recreates.into_iter().map(|(recreate, _)| recreate).collect(), weights }
    }
}

impl Recreate for CompositeRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        // NOTE always use recreate method with the larger weight for the initial generation
        let index = if refinement_ctx.generation == 1 { 0 } else { insertion_ctx.random.weighted(self.weights.iter()) };
        self.recreates.get(index).unwrap().run(refinement_ctx, insertion_ctx)
    }
}
