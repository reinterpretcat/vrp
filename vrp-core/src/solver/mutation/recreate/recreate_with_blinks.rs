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
use crate::utils::compare_floats;
use rand::prelude::*;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use std::sync::Arc;

struct DemandJobSelector<T: Load + Add<Output = T> + Sub<Output = T> + 'static> {
    asc_order: bool,
    phantom: PhantomData<T>,
}

impl<T: Load + Add<Output = T> + Sub<Output = T> + 'static> DemandJobSelector<T> {
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

impl<T: Load + Add<Output = T> + Sub<Output = T> + 'static> JobSelector for DemandJobSelector<T> {
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
            .map(|profile| problem.jobs.rank(*profile, job))
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
    ratio: f64,
}

impl Default for BlinkResultSelector {
    fn default() -> Self {
        Self { ratio: 0.01 }
    }
}

impl ResultSelector for BlinkResultSelector {
    fn select(&self, ctx: &InsertionContext, left: InsertionResult, right: InsertionResult) -> InsertionResult {
        let is_blink = ctx.environment.random.is_hit(self.ratio);
        let is_locked = match &right {
            InsertionResult::Success(success) => ctx.solution.locked.contains(&success.job),
            _ => false,
        };
        match (&left, is_blink, is_locked) {
            (InsertionResult::Success(_), true, false) => left,
            _ => InsertionResult::choose_best_result(left, right),
        }
    }
}

/// A recreate method as described in "Slack Induction by String Removals for
/// Vehicle Routing Problems" (aka SISR) paper by Jan Christiaens, Greet Vanden Berghe.
pub struct RecreateWithBlinks<T: Load + Add<Output = T> + Sub<Output = T> + 'static> {
    job_selectors: Vec<Box<dyn JobSelector + Send + Sync>>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
    weights: Vec<usize>,
    phantom: PhantomData<T>,
}

impl<T: Load + Add<Output = T> + Sub<Output = T> + 'static> RecreateWithBlinks<T> {
    /// Creates a new instance of `RecreateWithBlinks`.
    pub fn new(selectors: Vec<(Box<dyn JobSelector + Send + Sync>, usize)>) -> Self {
        let weights = selectors.iter().map(|(_, weight)| *weight).collect();
        Self {
            job_selectors: selectors.into_iter().map(|(selector, _)| selector).collect(),
            job_reducer: Box::new(PairJobMapReducer::new(
                Box::new(AllRouteSelector::default()),
                Box::new(BlinkResultSelector::default()),
            )),
            weights,
            phantom: PhantomData,
        }
    }
}

impl<T: Load + Add<Output = T> + Sub<Output = T> + 'static> Default for RecreateWithBlinks<T> {
    fn default() -> Self {
        Self::new(vec![
            (Box::new(AllJobSelector::default()), 10),
            (Box::new(ChunkJobSelector::new(8)), 10),
            (Box::new(DemandJobSelector::<T>::new(false)), 10),
            (Box::new(DemandJobSelector::<T>::new(true)), 1),
            (Box::new(RankedJobSelector::new(true)), 5),
            (Box::new(RankedJobSelector::new(false)), 1),
        ])
    }
}

impl<T: Load + Add<Output = T> + Sub<Output = T> + 'static> Recreate for RecreateWithBlinks<T> {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.environment.random.weighted(self.weights.as_slice());
        let job_selector = self.job_selectors.get(index).unwrap();
        InsertionHeuristic::default().process(
            job_selector.as_ref(),
            self.job_reducer.as_ref(),
            insertion_ctx,
            &refinement_ctx.quota,
        )
    }
}
