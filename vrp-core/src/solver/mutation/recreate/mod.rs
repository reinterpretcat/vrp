//! The recreate module contains logic to build a feasible solution from partially ruined.

use crate::construction::heuristics::*;
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

/// Provides the way to run one of multiple recreate methods.
pub struct WeightedRecreate {
    recreates: Vec<Arc<dyn Recreate + Send + Sync>>,
    weights: Vec<usize>,
}

impl WeightedRecreate {
    /// Creates a new instance of `WeightedRecreate` using list of recreate strategies.
    pub fn new(recreates: Vec<(Arc<dyn Recreate + Send + Sync>, usize)>) -> Self {
        let (recreates, weights) = recreates.into_iter().unzip();
        Self { recreates, weights }
    }
}

impl Recreate for WeightedRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        self.recreates.get(index).unwrap().run(refinement_ctx, insertion_ctx)
    }
}

/// Provides way to reuse generic behaviour.
pub struct ConfigurableRecreate {
    job_selector: Box<dyn JobSelector + Send + Sync>,
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    insertion_heuristic: InsertionHeuristic,
}

impl ConfigurableRecreate {
    /// Creates a new instance of `ConfigurableRecreate`.
    pub fn new(
        job_selector: Box<dyn JobSelector + Send + Sync>,
        route_selector: Box<dyn RouteSelector + Send + Sync>,
        result_selector: Box<dyn ResultSelector + Send + Sync>,
        insertion_heuristic: InsertionHeuristic,
    ) -> Self {
        Self { job_selector, route_selector, result_selector, insertion_heuristic }
    }
}

impl Recreate for ConfigurableRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.insertion_heuristic.process(
            insertion_ctx,
            self.job_selector.as_ref(),
            self.route_selector.as_ref(),
            self.result_selector.as_ref(),
            &refinement_ctx.quota,
        )
    }
}
