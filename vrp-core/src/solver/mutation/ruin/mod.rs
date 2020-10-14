//! The ruin module contains various strategies to destroy small, medium or large parts of an
//! existing solution.

use crate::construction::heuristics::InsertionContext;
use crate::models::Problem;
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

/// A type which specifies a group of multiple ruin strategies with its probability.
pub type RuinGroup = (Vec<(Arc<dyn Ruin + Send + Sync>, f64)>, usize);

/// Provides the way to run multiple ruin methods one by one on the same solution.
pub struct CompositeRuin {
    ruins: Vec<Vec<(Arc<dyn Ruin + Send + Sync>, f64)>>,
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

impl CompositeRuin {
    /// Creates a new instance of `CompositeRuin` with passed ruin methods.
    pub fn new(ruins: Vec<RuinGroup>) -> Self {
        let weights = ruins.iter().map(|(_, weight)| *weight).collect();
        let ruins = ruins.into_iter().map(|(ruin, _)| ruin).collect();

        Self { ruins, weights }
    }

    /// Creates a new instance of `CompositeRuin` with default ruin methods.
    pub fn new_from_problem(problem: Arc<Problem>) -> Self {
        let random_route = Arc::new(RandomRouteRemoval::default());
        let random_job = Arc::new(RandomJobRemoval::new(JobRemovalLimit::default()));

        Self::new(vec![
            (
                vec![
                    (Arc::new(AdjustedStringRemoval::default()), 1.),
                    (Arc::new(NeighbourRemoval::new(JobRemovalLimit::new(2, 8, 0.1))), 0.1),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                100,
            ),
            (
                vec![
                    (Arc::new(WorstJobRemoval::default()), 1.),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                10,
            ),
            (
                vec![
                    (Arc::new(NeighbourRemoval::default()), 1.),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                10,
            ),
            (vec![(random_job.clone(), 1.), (random_route.clone(), 0.1)], 2),
            (vec![(random_route.clone(), 1.), (random_job.clone(), 0.1)], 2),
            (
                vec![
                    (Arc::new(ClusterRemoval::new_with_defaults(problem)), 1.),
                    (random_job, 0.05),
                    (random_route, 0.01),
                ],
                1,
            ),
        ])
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.random.clone();

        let index = insertion_ctx.random.weighted(self.weights.as_slice());

        let mut insertion_ctx = self
            .ruins
            .get(index)
            .unwrap()
            .iter()
            .filter(|(_, probability)| random.is_hit(*probability))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}
