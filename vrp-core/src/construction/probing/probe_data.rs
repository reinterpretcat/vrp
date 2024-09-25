#![allow(dead_code)]
#![allow(missing_docs)]

use crate::construction::heuristics::InsertionResult;
use crate::models::problem::{Actor, VehicleIdDimension};
use crate::prelude::*;
use crate::utils::BloomFilter;
use rosomaxa::prelude::short_type_name;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

custom_tour_state!(RouteProbe typeof RouteProbe);

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
#[derive(Clone)]
pub struct RouteProbe {
    actor: Arc<Actor>,
    job_size: usize,
    filter: BloomFilter<Job>,
}

impl RouteProbe {
    pub(crate) fn new_filter(actor: Arc<Actor>, job_size: usize, seed: u64) -> Self {
        Self { actor, job_size, filter: BloomFilter::new_with_seed(job_size, 0.05, (seed, seed / 2)) }
    }

    pub(crate) fn insert(&mut self, job: &Job) {
        self.filter.insert(job);
    }

    pub(crate) fn contains(&self, job: &Job) -> bool {
        self.filter.contains(job)
    }

    pub(crate) fn eval_job<F>(&mut self, job: &Job, init_result: InsertionResult, eval_fn: F) -> InsertionResult
    where
        F: FnOnce(InsertionResult) -> InsertionResult,
    {
        if !self.contains(job) {
            let new_result = eval_fn(init_result);

            if matches!(new_result, InsertionResult::Failure(_)) {
                self.insert(job);
            }

            new_result
        } else {
            init_result
        }
    }

    pub(crate) fn merge(&mut self, other: &RouteProbe) {
        self.filter.union(&other.filter);
    }
}

pub(crate) trait ProbeDataExt {
    fn take_route_probe(&mut self, route_ctx: &RouteContext, job_size: usize, seed: u64) -> RouteProbe;

    fn attach_route_probe(self, probe_data: RouteProbe) -> Self;

    fn attach_probe_data(self, probe_data: ProbeData) -> Self;

    fn merge_probe_data(&mut self, other: &mut Self, job_size: usize) -> ProbeData;
}

impl ProbeDataExt for InsertionResult {
    fn take_route_probe(&mut self, route_ctx: &RouteContext, job_size: usize, seed: u64) -> RouteProbe {
        let actor = &route_ctx.route().actor;
        self.get_probe_data_mut()
            .take()
            .and_then(|probe| probe.routes.remove(actor))
            .or_else(|| route_ctx.state().get_route_probe().cloned())
            .unwrap_or_else(|| RouteProbe::new_filter(actor.clone(), job_size, seed))
    }

    fn attach_route_probe(mut self, route_probe: RouteProbe) -> Self {
        if let Some(probe_data) = self.get_probe_data_mut() {
            probe_data.routes.insert(route_probe.actor.clone(), route_probe);

            self
        } else {
            let mut probe_data = ProbeData::new(route_probe.job_size);
            probe_data.routes.insert(route_probe.actor.clone(), route_probe);

            self.attach_probe_data(probe_data)
        }
    }

    fn attach_probe_data(mut self, new_probe_data: ProbeData) -> Self {
        let old_probe_data = self.take_probe_data();

        let probe_data = if let Some(old_probe_data) = old_probe_data {
            // TODO: that could be expensive, need to minimize somehow?
            old_probe_data.merge(new_probe_data)
        } else {
            new_probe_data
        };

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

impl Debug for ProbeData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let unknown_id = "?".to_string();
        f.debug_struct(short_type_name::<Self>())
            .field(
                "routes",
                &self
                    .routes
                    .keys()
                    // TODO add display for values
                    .map(|a| a.vehicle.dimens.get_vehicle_id().unwrap_or(&unknown_id))
                    .collect::<Vec<_>>(),
            )
            .field("job_size", &self.job_size)
            .finish()
    }
}
