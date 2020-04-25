#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/recreate/recreate_with_blinks_test.rs"]
mod recreate_with_blinks_test;

extern crate rand;

use crate::construction::constraints::{Demand, DemandDimension};
use crate::construction::heuristics::*;
use crate::construction::heuristics::{InsertionContext, InsertionResult};
use crate::models::common::Distance;
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

struct DemandJobSelector<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    asc_order: bool,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    DemandJobSelector<Capacity>
{
    pub fn new(asc_order: bool) -> Self {
        Self { asc_order, phantom: PhantomData }
    }

    fn get_capacity(demand: &Demand<Capacity>) -> Capacity {
        demand.pickup.0 + demand.delivery.0 + demand.pickup.1 + demand.delivery.1
    }

    fn get_job_demand(job: &Job) -> Option<Capacity> {
        match job {
            Job::Single(job) => job.dimens.get_demand(),
            Job::Multi(job) => job.jobs.first().and_then(|s| s.dimens.get_demand()),
        }
        .map(|d| Self::get_capacity(d))
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    JobSelector for DemandJobSelector<Capacity>
{
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

struct RandomJobSelector {}

impl RandomJobSelector {
    pub fn new() -> Self {
        Self {}
    }
}

impl JobSelector for RandomJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Job> + 'a> {
        ctx.solution.required.shuffle(&mut rand::thread_rng());

        Box::new(ctx.solution.required.iter().cloned())
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

/// Selects best result with "blinks" - select random one with some probability.
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
        let is_blink = ctx.random.uniform_real(0., 1.) < self.ratio;
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
pub struct RecreateWithBlinks<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    job_selectors: Vec<Box<dyn JobSelector + Send + Sync>>,
    job_reducer: Box<dyn JobMapReducer + Send + Sync>,
    weights: Vec<usize>,
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    RecreateWithBlinks<Capacity>
{
    pub fn new(selectors: Vec<(Box<dyn JobSelector + Send + Sync>, usize)>) -> Self {
        let weights = selectors.iter().map(|(_, weight)| *weight).collect();
        Self {
            job_selectors: selectors.into_iter().map(|(selector, _)| selector).collect(),
            job_reducer: Box::new(PairJobMapReducer::new(Box::new(BlinkResultSelector::default()))),
            weights,
            phantom: PhantomData,
        }
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Default
    for RecreateWithBlinks<Capacity>
{
    fn default() -> Self {
        Self::new(vec![
            (Box::new(RandomJobSelector::new()), 10),
            (Box::new(DemandJobSelector::<Capacity>::new(false)), 10),
            (Box::new(DemandJobSelector::<Capacity>::new(true)), 1),
            (Box::new(RankedJobSelector::new(true)), 5),
            (Box::new(RankedJobSelector::new(false)), 1),
        ])
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Recreate
    for RecreateWithBlinks<Capacity>
{
    fn run(&self, refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let index = insertion_ctx.random.weighted(self.weights.as_slice());
        let job_selector = self.job_selectors.get(index).unwrap();
        InsertionHeuristic::default().process(&job_selector, &self.job_reducer, insertion_ctx, &refinement_ctx.quota)
    }
}
