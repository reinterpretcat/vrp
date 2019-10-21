#[cfg(test)]
#[path = "../../../tests/unit/refinement/recreate/recreate_with_blinks_test.rs"]
mod recreate_with_blinks_test;

extern crate rand;

use crate::construction::constraints::{CapacityDimension, Demand, DemandDimension};
use crate::construction::heuristics::JobSelector;
use crate::construction::states::InsertionContext;
use crate::models::problem::Job;
use crate::refinement::recreate::Recreate;
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

impl JobSelector for RandomJobSelector {
    fn select<'a>(&'a self, ctx: &'a mut InsertionContext) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
        ctx.solution.required.shuffle(&mut rand::thread_rng());

        Box::new(ctx.solution.required.iter().cloned())
    }
}

struct RankedJobSelector {}

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
