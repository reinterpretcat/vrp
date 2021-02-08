//! The ruin module contains various strategies to destroy small, medium or large parts of an
//! existing solution.

use crate::construction::heuristics::InsertionContext;
use crate::solver::RefinementContext;
use std::iter::once;
use std::sync::Arc;

/// A trait which specifies logic to destroy parts of solution.
pub trait Ruin {
    /// Ruins given solution and returns a new one with less jobs assigned.
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod adjusted_string_removal;
pub use self::adjusted_string_removal::AdjustedStringRemoval;

mod cluster_removal;
pub use self::cluster_removal::ClusterRemoval;

mod neighbour_removal;
pub use self::neighbour_removal::NeighbourRemoval;

mod random_route_removal;
pub use self::random_route_removal::RandomRouteRemoval;

mod random_job_removal;
pub use self::random_job_removal::RandomJobRemoval;

mod worst_jobs_removal;
pub use self::worst_jobs_removal::WorstJobRemoval;

/// A type which specifies a group of multiple ruin strategies with their probability.
pub type RuinGroup = (Vec<(Arc<dyn Ruin + Send + Sync>, f64)>, usize);

/// Provides the way to pick one ruin from the group ruin methods.
pub struct WeightedRuin {
    ruins: Vec<CompositeRuin>,
    weights: Vec<usize>,
}

/// Specifies a limit for amount of jobs to be removed.
pub struct JobRemovalLimit {
    /// Specifies minimum amount of removed jobs.
    pub min: usize,
    /// Specifies maximum amount of removed jobs.
    pub max: usize,
    /// Specifies threshold ratio of maximum removed jobs.
    pub threshold: f64,
}

impl JobRemovalLimit {
    /// Creates a new instance of `JobRemovalLimit`.
    pub fn new(min: usize, max: usize, threshold: f64) -> Self {
        Self { min, max, threshold }
    }
}

impl Default for JobRemovalLimit {
    fn default() -> Self {
        Self { min: 8, max: 16, threshold: 0.1 }
    }
}

impl WeightedRuin {
    /// Creates a new instance of `WeightedRuin` with passed ruin methods.
    pub fn new(ruins: Vec<RuinGroup>) -> Self {
        let weights = ruins.iter().map(|(_, weight)| *weight).collect();
        let ruins = ruins.into_iter().map(|(ruin, _)| CompositeRuin::new(ruin)).collect();

        Self { ruins, weights }
    }
}

impl Ruin for WeightedRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());

        self.ruins[index].run(refinement_ctx, insertion_ctx)
    }
}

/// Provides the way to run multiple ruin methods one by one on the same solution.
pub struct CompositeRuin {
    ruins: Vec<(Arc<dyn Ruin + Send + Sync>, f64)>,
}

impl CompositeRuin {
    /// Creates a new instance of `CompositeRuin` using list of ruin strategies.
    pub fn new(ruins: Vec<(Arc<dyn Ruin + Send + Sync>, f64)>) -> Self {
        Self { ruins }
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.environment.random.clone();

        let mut insertion_ctx = self
            .ruins
            .iter()
            .filter(|(_, probability)| random.is_hit(*probability))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}
