#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/recreate/recreate_with_blinks_test.rs"]
mod recreate_with_blinks_test;

use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::common::*;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::mutation::recreate::Recreate;
use crate::solver::RefinementContext;
use crate::utils::Either;
use rand::prelude::*;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::sync::Arc;

struct DemandJobSelector<T: LoadOps> {
    asc_order: bool,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> DemandJobSelector<T> {
    pub fn new(asc_order: bool) -> Self {
        Self { asc_order, phantom: PhantomData }
    }

    fn get_capacity(demand: &Demand<T>) -> T {
        demand.pickup.0 + demand.delivery.0 + demand.pickup.1 + demand.delivery.1
    }

    fn get_job_demand(job: &Job) -> Option<T> {
        match job {
            Job::Single(job) => job.dimens.get_demand(),
            Job::Multi(job) => job.jobs.first().and_then(|s| s.dimens.get_demand()),
        }
        .map(|d| Self::get_capacity(d))
    }
}

impl<T: LoadOps> JobSelector for DemandJobSelector<T> {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.sort_by(|a, b| match (Self::get_job_demand(a), Self::get_job_demand(b)) {
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a), Some(b)) => b.cmp(&a),
            (None, None) => Ordering::Equal,
        });

        if self.asc_order {
            ctx.solution.required.reverse();
        }

        Box::new(ctx.solution.required.iter().cloned())
    }
}

struct ChunkJobSelector {
    size: usize,
}

impl ChunkJobSelector {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

impl JobSelector for ChunkJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut ctx.environment.random.get_rng());

        Box::new(ctx.solution.required.iter().take(self.size).cloned())
    }
}

struct RankedJobSelector {
    asc_order: bool,
}

impl RankedJobSelector {
    pub fn new(asc_order: bool) -> Self {
        Self { asc_order }
    }

    pub fn rank_job(problem: &Arc<Problem>, job: &Job) -> Distance {
        problem
            .fleet
            .profiles
            .iter()
            .map(|profile| problem.jobs.rank(profile, job))
            .min_by(|a, b| compare_floats(*a, *b))
            .unwrap_or_default()
    }
}

impl JobSelector for RankedJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        let problem = &ctx.problem;

        ctx.solution.required.sort_by(|a, b| {
            Self::rank_job(problem, a).partial_cmp(&Self::rank_job(problem, b)).unwrap_or(Ordering::Less)
        });

        if self.asc_order {
            ctx.solution.required.reverse();
        }

        Box::new(ctx.solution.required.iter().cloned())
    }
}

/// A recreate strategy with blinks inspired by "Slack Induction by String Removals for Vehicle
/// Routing Problems", Jan Christiaens, Greet Vanden Berghe.
struct BlinkResultSelector {
    random: Arc<dyn Random + Send + Sync>,
    ratio: f64,
}

impl BlinkResultSelector {
    /// Creates an instance of `BlinkResultSelector`.
    fn new(ratio: f64, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { random, ratio }
    }

    /// Creates an instance of `BlinkResultSelector` with default values.
    fn new_with_defaults(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self::new(0.01, random)
    }
}

impl ResultSelector for BlinkResultSelector {
    fn select_insertion(
        &self,
        ctx: &InsertionContext,
        left: InsertionResult,
        right: InsertionResult,
    ) -> InsertionResult {
        let is_blink = self.random.is_hit(self.ratio);
        let is_locked = match &right {
            InsertionResult::Success(success) => ctx.solution.locked.contains(&success.job),
            _ => false,
        };
        match (&left, is_blink, is_locked) {
            (InsertionResult::Success(_), true, false) => left,
            _ => InsertionResult::choose_best_result(left, right),
        }
    }

    fn select_cost(&self, _route_ctx: &RouteContext, left: f64, right: f64) -> Either {
        let is_blink = self.random.is_hit(self.ratio);

        if is_blink || left < right {
            Either::Left
        } else {
            Either::Right
        }
    }
}

/// A recreate method as described in "Slack Induction by String Removals for
/// Vehicle Routing Problems" (aka SISR) paper by Jan Christiaens, Greet Vanden Berghe.
pub struct RecreateWithBlinks<T: LoadOps> {
    job_selectors: Vec<Box<dyn JobSelector + Send + Sync>>,
    route_selector: Box<dyn RouteSelector + Send + Sync>,
    leg_selector: Box<dyn LegSelector + Send + Sync>,
    result_selector: Box<dyn ResultSelector + Send + Sync>,
    insertion_heuristic: InsertionHeuristic,
    weights: Vec<usize>,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> RecreateWithBlinks<T> {
    /// Creates a new instance of `RecreateWithBlinks`.
    pub fn new(
        selectors: Vec<(Box<dyn JobSelector + Send + Sync>, usize)>,
        random: Arc<dyn Random + Send + Sync>,
    ) -> Self {
        let weights = selectors.iter().map(|(_, weight)| *weight).collect();
        Self {
            job_selectors: selectors.into_iter().map(|(selector, _)| selector).collect(),
            route_selector: Box::new(AllRouteSelector::default()),
            leg_selector: Box::new(VariableLegSelector::new(random.clone())),
            result_selector: Box::new(BlinkResultSelector::new_with_defaults(random)),
            insertion_heuristic: Default::default(),
            weights,
            phantom: PhantomData,
        }
    }

    /// Creates a new instance of `RecreateWithBlinks` with default prameters.
    pub fn new_with_defaults(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self::new(
            vec![
                (Box::new(AllJobSelector::default()), 10),
                (Box::new(ChunkJobSelector::new(8)), 10),
                (Box::new(DemandJobSelector::<T>::new(false)), 10),
                (Box::new(DemandJobSelector::<T>::new(true)), 1),
                (Box::new(RankedJobSelector::new(true)), 5),
                (Box::new(RankedJobSelector::new(false)), 1),
            ],
            random,
        )
    }
}

impl<T: LoadOps> Recreate for RecreateWithBlinks<T> {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        let job_selector = self.job_selectors.get(index).unwrap().as_ref();

        self.insertion_heuristic.process(
            insertion_ctx,
            job_selector,
            self.route_selector.as_ref(),
            self.leg_selector.as_ref(),
            self.result_selector.as_ref(),
            &refinement_ctx.quota,
        )
    }
}
