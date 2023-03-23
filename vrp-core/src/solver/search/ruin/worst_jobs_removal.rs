#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/worst_jobs_removal_test.rs"]
mod worst_jobs_removal_test;

use super::*;
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{Job, TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};
use crate::solver::search::{get_route_jobs, JobRemovalTracker};
use crate::solver::RefinementContext;
use hashbrown::HashMap;
use rosomaxa::utils::parallel_collect;
use std::cmp::Ordering::Less;
use std::iter::once;
use std::sync::Arc;

/// A ruin strategy which detects the most cost expensive jobs in each route and delete them
/// with their neighbours.
pub struct WorstJobRemoval {
    /// Specifies limitation for job removal.
    limits: RemovalLimits,
    /// Amount of jobs to skip.
    worst_skip: usize,
}

impl WorstJobRemoval {
    /// Creates a new instance of `WorstJobRemoval`.
    pub fn new(worst_skip: usize, limits: RemovalLimits) -> Self {
        Self { limits, worst_skip }
    }
}

impl Ruin for WorstJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.environment.random.clone();
        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let mut routes_savings = get_routes_cost_savings(&insertion_ctx);

        routes_savings.shuffle(&mut random.get_rng());

        let tracker = RwLock::new(JobRemovalTracker::new(&self.limits, random.as_ref()));

        routes_savings.iter().take_while(|_| !tracker.read().unwrap().is_limit()).for_each(|(route_ctx, savings)| {
            let skip = savings.len().min(random.uniform_int(0, self.worst_skip as i32) as usize);
            let worst = savings
                .iter()
                .filter(|(job, _)| {
                    let solution = &insertion_ctx.solution;
                    !solution.locked.contains(job) && !solution.unassigned.contains_key(job)
                })
                .nth(skip);

            if let Some((job, _)) = worst {
                once(job.clone())
                    .chain(
                        problem
                            .jobs
                            .neighbors(&route_ctx.route.actor.vehicle.profile, job, Timestamp::default())
                            .filter(|(_, cost)| *cost > 0.)
                            .map(|(job, _)| job)
                            .cloned(),
                    )
                    .take_while(|_| !tracker.read().unwrap().is_limit())
                    .for_each(|job| {
                        // NOTE job can be absent if it is unassigned
                        if let Some(route_ctx) = route_jobs.get_mut(&job) {
                            tracker.write().unwrap().try_remove_job(&mut insertion_ctx.solution, route_ctx, &job);
                        }
                    });
            }
        });

        insertion_ctx
    }
}

fn get_routes_cost_savings(insertion_ctx: &InsertionContext) -> Vec<(RouteContext, Vec<(Job, Cost)>)> {
    parallel_collect(&insertion_ctx.solution.routes, |route_ctx| {
        let route = route_ctx.route.as_ref();
        let mut savings: Vec<(Job, Cost)> = route
            .tour
            .all_activities()
            .as_slice()
            .windows(3)
            .fold(HashMap::<Job, Cost>::default(), |mut acc, iter| match iter {
                [start, eval, end] => {
                    let savings = get_cost_savings(route, start, eval, end, &insertion_ctx.problem.transport);
                    let job = eval.retrieve_job().unwrap_or_else(|| panic!("Unexpected activity without job"));
                    *acc.entry(job).or_insert(0.) += savings;

                    acc
                }
                _ => panic!("Unexpected activity window"),
            })
            .drain()
            .collect();
        savings.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Less));

        (route_ctx.clone(), savings)
    })
}

fn get_cost_savings(
    route: &Route,
    start: &Activity,
    middle: &Activity,
    end: &Activity,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> Cost {
    let actor = route.actor.as_ref();

    let waiting_costs = (middle.place.time.start - middle.schedule.arrival).max(0.)
        * (actor.driver.costs.per_waiting_time + actor.vehicle.costs.per_waiting_time);

    let transport_costs = get_cost(route, start, middle, transport) + get_cost(route, middle, end, transport)
        - get_cost(route, start, end, transport);

    waiting_costs + transport_costs
}

#[inline(always)]
fn get_cost(route: &Route, from: &Activity, to: &Activity, transport: &Arc<dyn TransportCost + Send + Sync>) -> Cost {
    transport.cost(route, from.place.location, to.place.location, TravelTime::Departure(from.schedule.departure))
}
