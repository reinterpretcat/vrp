#[cfg(test)]
#[path = "../../../tests/unit/refinement/recreate/recreate_with_blinks_test.rs"]
mod recreate_with_blinks_test;

extern crate rand;

use crate::construction::constraints::{CapacityDimension, Demand, DemandDimension};
use crate::construction::heuristics::JobSelector;
use crate::construction::states::InsertionContext;
use crate::models::common::Distance;
use crate::models::problem::Job;
use crate::models::Problem;
use crate::refinement::recreate::Recreate;
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

    fn get_job_demand(job: &Arc<Job>) -> Option<Capacity> {
        match job.as_ref() {
            Job::Single(job) => job.dimens.get_demand(),
            Job::Multi(job) => job.jobs.first().and_then(|s| s.dimens.get_demand()),
        }
        .and_then(|d| Some(Self::get_capacity(d)))
    }
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static>
    JobSelector for DemandJobSelector<Capacity>
{
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
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
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
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

    pub fn rank_job(problem: &Arc<Problem>, job: &Arc<Job>) -> Distance {
        problem
            .fleet
            .profiles
            .iter()
            .map(|profile| problem.jobs.rank(*profile, job))
            .min_by(|a, b| compare_floats(a, b))
            .unwrap_or(Distance::default())
    }
}

impl JobSelector for RankedJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
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

pub struct RecreateWithBlinks<Capacity: Add + Sub + Ord + Copy + Default + Send + Sync + 'static> {
    phantom: PhantomData<Capacity>,
}

impl<Capacity: Add<Output = Capacity> + Sub<Output = Capacity> + Ord + Copy + Default + Send + Sync + 'static> Recreate
    for RecreateWithBlinks<Capacity>
{
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        unimplemented!()
    }
}
