#![allow(missing_docs)]

use crate::construction::heuristics::{InsertionFailure, InsertionResult};
use crate::models::problem::Actor;
use crate::prelude::*;
use crate::utils::BloomFilter;
use std::collections::HashMap;
use std::sync::Arc;

custom_tour_state!(RouteProbe typeof RouteProbe);

#[doc(hidden)]
#[derive(Debug)]
pub struct ProbeData {
    routes: HashMap<Arc<Actor>, RouteProbe>,
    job_size: usize,
}

impl ProbeData {
    pub(crate) fn new(job_size: usize) -> Self {
        Self { routes: Default::default(), job_size }
    }

    pub(crate) fn attach(&mut self, solution_ctx: &mut SolutionContext) {
        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            let is_stale = route_ctx.is_stale();
            if let Some(probe) = self.routes.remove(&route_ctx.route().actor) {
                route_ctx.state_mut().set_route_probe(probe);
            }

            route_ctx.mark_stale(is_stale);
        });
    }

    pub(crate) fn insert(&mut self, actor: Arc<Actor>, job: &Job) {
        self.routes.entry(actor).or_insert_with(|| RouteProbe::new_filter(self.job_size, 0)).insert(job);
    }

    pub(crate) fn remove(&mut self, actor: &Arc<Actor>) {
        self.routes.remove(actor);
    }

    fn merge(self, other: Self) -> Self {
        let (mut source, mut destination) =
            if self.routes.len() > other.routes.len() { (other, self) } else { (self, other) };

        for (actor, src_route_probe) in std::mem::take(&mut source.routes) {
            if let Some(dest_route_probe) = destination.routes.get_mut(&actor) {
                dest_route_probe.merge(&src_route_probe);
            } else {
                destination.routes.insert(actor, src_route_probe);
            }
        }

        destination
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct RouteProbe {
    filter: BloomFilter<Job>,
}

impl RouteProbe {
    pub(crate) fn new_filter(job_size: usize, seed: u64) -> Self {
        Self { filter: BloomFilter::new_with_seed(job_size, 0.05, (seed, seed / 2)) }
    }

    pub(crate) fn insert(&mut self, job: &Job) {
        self.filter.insert(job);
    }

    pub(crate) fn contains(&self, job: &Job) -> bool {
        self.filter.contains(job)
    }

    pub(crate) fn eval_job<F>(&self, job: &Job, init_result: InsertionResult, eval_fn: F) -> InsertionResult
    where
        F: FnOnce(InsertionResult) -> InsertionResult,
    {
        if self.contains(job) {
            init_result
        } else {
            eval_fn(init_result)
        }
    }

    pub(crate) fn merge(&mut self, other: &RouteProbe) {
        self.filter.union(&other.filter);
    }
}

pub(crate) trait InsertionProbe {
    fn attach_probe_data(self, probe_data: ProbeData) -> Self;

    fn merge_probe_data(&mut self, other: &mut Self, job_size: usize) -> ProbeData;
}

impl InsertionProbe for InsertionResult {
    fn attach_probe_data(mut self, probe_data: ProbeData) -> Self {
        let mut probe_data = match self.take_probe_data() {
            Some(data) => data.merge(probe_data),
            None => probe_data,
        };

        if let InsertionResult::Failure(InsertionFailure { job: Some(job), actor: Some(actor), .. }) = &self {
            probe_data.insert(actor.clone(), job);
        }

        match &mut self {
            InsertionResult::Success(success) => success.probe = Some(probe_data),
            InsertionResult::Failure(failure) => failure.probe = Some(probe_data),
        }

        self
    }

    fn merge_probe_data(&mut self, other: &mut Self, job_size: usize) -> ProbeData {
        let left = self.take_probe_data();
        let right = other.take_probe_data();

        match (left, right) {
            (Some(left), Some(right)) => left.merge(right),
            (Some(left), None) => left,
            (None, Some(right)) => right,
            (None, None) => ProbeData::new(job_size),
        }
    }
}
