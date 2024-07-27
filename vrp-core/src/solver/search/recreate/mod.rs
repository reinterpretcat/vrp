//! The recreate module contains logic to build a feasible solution from partially ruined.

use crate::construction::heuristics::*;
use crate::models::{GoalContext, Problem};
use crate::solver::RefinementContext;
use rosomaxa::prelude::SelectionPhase;
use rosomaxa::HeuristicContext;
use std::collections::HashMap;
use std::sync::Arc;

/// A trait which specifies logic to produce a new feasible solution from partial one.
pub trait Recreate: Send + Sync {
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

mod recreate_with_regret;
pub use self::recreate_with_regret::RecreateWithRegret;

mod recreate_with_skip_best;
pub use self::recreate_with_skip_best::RecreateWithSkipBest;

mod recreate_with_skip_random;
pub use self::recreate_with_skip_random::RecreateWithSkipRandom;

mod recreate_with_slice;
pub use self::recreate_with_slice::RecreateWithSlice;

/// Provides the way to run one of multiple recreate methods.
pub struct WeightedRecreate {
    recreates: Vec<Arc<dyn Recreate>>,
    weights: Vec<usize>,
}

impl WeightedRecreate {
    /// Creates a new instance of `WeightedRecreate` using list of recreate strategies.
    pub fn new(recreates: Vec<(Arc<dyn Recreate>, usize)>) -> Self {
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
    job_selector: Box<dyn JobSelector>,
    route_selector: Box<dyn RouteSelector>,
    leg_selection: LegSelection,
    result_selection: ResultSelection,
    insertion_heuristic: InsertionHeuristic,
}

impl ConfigurableRecreate {
    /// Creates a new instance of `ConfigurableRecreate`.
    pub fn new(
        job_selector: Box<dyn JobSelector>,
        route_selector: Box<dyn RouteSelector>,
        leg_selection: LegSelection,
        result_selection: ResultSelection,
        insertion_heuristic: InsertionHeuristic,
    ) -> Self {
        Self { job_selector, route_selector, leg_selection, result_selection, insertion_heuristic }
    }
}

impl Recreate for ConfigurableRecreate {
    fn run(&self, _: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let result_selector = match &self.result_selection {
            ResultSelection::Concrete(concrete) => concrete.as_ref(),
            ResultSelection::Stochastic(provider) => provider.pick(),
        };

        self.insertion_heuristic.process(
            insertion_ctx,
            self.job_selector.as_ref(),
            self.route_selector.as_ref(),
            &self.leg_selection,
            result_selector,
        )
    }
}

/// Provides way to use different recreate methods on different selection phases.
pub struct PhasedRecreate {
    recreates: HashMap<SelectionPhase, Arc<dyn Recreate>>,
}

impl PhasedRecreate {
    /// Creates a new instance of `PhasedRecreate`.
    pub fn new(recreates: HashMap<SelectionPhase, Arc<dyn Recreate>>) -> Self {
        assert!([SelectionPhase::Initial, SelectionPhase::Exploration, SelectionPhase::Exploitation]
            .iter()
            .all(|key| recreates.contains_key(key)));

        Self { recreates }
    }
}

impl Recreate for PhasedRecreate {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        self.recreates.get(&refinement_ctx.selection_phase()).unwrap().run(refinement_ctx, insertion_ctx)
    }
}

pub(crate) struct RecreateWithGoal<T: Recreate> {
    goal: Arc<GoalContext>,
    inner: T,
}

impl<T: Recreate> RecreateWithGoal<T> {
    /// Creates a new instance of `RecreateWithGoal`.
    pub fn new(goal: Arc<GoalContext>, inner: T) -> Self {
        Self { goal, inner }
    }
}

impl<T: Recreate> Recreate for RecreateWithGoal<T> {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();

        let insertion_ctx = InsertionContext {
            problem: Arc::new(Problem {
                fleet: problem.fleet.clone(),
                jobs: problem.jobs.clone(),
                locks: problem.locks.clone(),
                goal: self.goal.clone(),
                activity: problem.activity.clone(),
                transport: problem.transport.clone(),
                extras: problem.extras.clone(),
            }),
            ..insertion_ctx
        };

        let mut insertion_ctx = self.inner.run(refinement_ctx, insertion_ctx);

        insertion_ctx.problem = problem;

        insertion_ctx
    }
}
