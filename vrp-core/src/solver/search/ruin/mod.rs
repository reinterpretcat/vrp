//! The ruin module contains various strategies to destroy small, medium or large parts of an
//! existing solution.

use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use crate::solver::RefinementContext;
use hashbrown::HashSet;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::*;
use std::sync::{Arc, RwLock};

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

mod route_removal;
pub use self::route_removal::*;

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
pub struct RuinLimits {
    /// Specifies minimum amount of ruined (removed) jobs.
    pub min_ruined_jobs: usize,
    /// Specifies maximum amount of ruined (removed) jobs.
    pub max_ruined_activities: usize,
    /// Specifies threshold for amount of ruined (removed) jobs.
    pub ruined_activities_threshold: f64,
    /// Specifies maximum amount of affected routes.
    pub max_affected_routes: usize,
}

impl RuinLimits {
    /// Creates a new instance of `RuinLimits`.
    pub fn new(
        min_ruined_activities: usize,
        max_ruined_activities: usize,
        ruined_jobs_threshold: f64,
        max_affected_routes: usize,
    ) -> Self {
        Self {
            min_ruined_jobs: min_ruined_activities,
            max_ruined_activities,
            ruined_activities_threshold: ruined_jobs_threshold,
            max_affected_routes,
        }
    }

    /// Gets chunk size based on limits.
    pub fn get_chunk_size(&self, ctx: &InsertionContext) -> usize {
        let total = ctx.problem.jobs.size() - ctx.solution.unassigned.len() - ctx.solution.ignored.len();

        let max_limit = (total as f64 * self.ruined_activities_threshold)
            .max(self.min_ruined_jobs as f64)
            .min(self.max_ruined_activities as f64)
            .round() as usize;

        ctx.environment
            .random
            .uniform_int(self.min_ruined_jobs as i32, self.max_ruined_activities as i32)
            .min(max_limit as i32) as usize
    }

    /// Gets a tracker of affected routes and jobs.
    pub(crate) fn get_tracker(&self) -> AffectedTracker {
        AffectedTracker {
            affected_actors: RwLock::new(HashSet::default()),
            removed_jobs: RwLock::new(HashSet::default()),
            limits: self,
        }
    }
}

impl Default for RuinLimits {
    fn default() -> Self {
        Self { min_ruined_jobs: 8, max_ruined_activities: 16, ruined_activities_threshold: 0.1, max_affected_routes: 8 }
    }
}

pub(crate) struct AffectedTracker<'a> {
    affected_actors: RwLock<HashSet<Arc<Actor>>>,
    removed_jobs: RwLock<HashSet<Job>>,
    limits: &'a RuinLimits,
}

impl<'a> AffectedTracker<'a> {
    pub fn add_job(&self, job: Job) {
        self.removed_jobs.write().unwrap().insert(job);
    }

    pub fn add_actor(&self, actor: Arc<Actor>) {
        self.affected_actors.write().unwrap().insert(actor);
    }

    pub fn is_affected_actor(&self, actor: &Actor) -> bool {
        self.affected_actors.read().unwrap().contains(actor)
    }

    pub fn is_removed_job(&self, job: &Job) -> bool {
        self.removed_jobs.read().unwrap().contains(job)
    }

    pub fn is_not_limit(&self, max_removed_activities: usize) -> bool {
        let removed_activities = self.get_removed_activities();
        let affected_routes = self.get_affected_actors();

        removed_activities < self.limits.max_ruined_activities
            && removed_activities < max_removed_activities
            && affected_routes < self.limits.max_affected_routes
    }

    pub fn get_affected_actors(&self) -> usize {
        self.affected_actors.read().unwrap().len()
    }

    pub fn get_removed_activities(&self) -> usize {
        // TODO cache value?
        self.removed_jobs.read().unwrap().iter().fold(0, |acc, job| {
            acc + match &job {
                Job::Single(_) => 1,
                Job::Multi(multi) => multi.jobs.len(),
            }
        })
    }

    pub fn iterate_removed_jobs<F: FnMut(&Job)>(&self, func: F) {
        self.removed_jobs.read().unwrap().iter().for_each(func)
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
