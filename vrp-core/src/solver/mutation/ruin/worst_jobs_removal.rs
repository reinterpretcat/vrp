#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/worst_jobs_removal_test.rs"]
mod worst_jobs_removal_test;

use super::*;
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::models::common::{Cost, Timestamp};
use crate::models::problem::{Actor, Job, TransportCost};
use crate::models::solution::Activity;
use crate::solver::mutation::get_route_jobs;
use crate::solver::RefinementContext;
use crate::utils::parallel_collect;
use hashbrown::HashMap;
use rand::prelude::*;
use std::cmp::Ordering::Less;
use std::iter::once;
use std::sync::Arc;

/// A ruin strategy which detects the most cost expensive jobs in each route and delete them
/// with their neighbours.
pub struct WorstJobRemoval {
    /// Specifies limitation for job removal.
    limits: RuinLimits,
    /// Amount of jobs to skip.
    worst_skip: usize,
}

impl WorstJobRemoval {
    /// Creates a new instance of `WorstJobRemoval`.
    pub fn new(worst_skip: usize, limits: RuinLimits) -> Self {
        Self { limits, worst_skip }
    }
}

impl Default for WorstJobRemoval {
    fn default() -> Self {
        Self::new(4, RuinLimits::default())
    }
}

impl Ruin for WorstJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.environment.random.clone();

        let can_remove_job = |job: &Job| -> bool {
            let solution = &insertion_ctx.solution;
            !solution.locked.contains(job) && !solution.unassigned.contains_key(job)
        };

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let mut routes_savings = get_routes_cost_savings(&insertion_ctx);

        routes_savings.shuffle(&mut random.get_rng());

        let max_removed_activities = self.limits.get_chunk_size(&insertion_ctx);
        let tracker = self.limits.get_tracker();

        routes_savings.iter().take_while(|_| tracker.is_not_limit(max_removed_activities)).for_each(|(rc, savings)| {
            let skip = savings.len().min(random.uniform_int(0, self.worst_skip as i32) as usize);
            let worst = savings.iter().filter(|(job, _)| can_remove_job(job)).nth(skip);

            if let Some((job, _)) = worst {
                let remove = self.limits.get_chunk_size(&insertion_ctx);
                once(job.clone())
                    .chain(
                        problem
                            .jobs
                            .neighbors(&rc.route.actor.vehicle.profile, &job, Timestamp::default())
                            .filter(|(_, cost)| *cost > 0.)
                            .map(|(job, _)| job)
                            .cloned(),
                    )
                    .filter(|job| can_remove_job(job))
                    .take_while(|_| tracker.is_not_limit(max_removed_activities))
                    .take(remove)
                    .for_each(|job| {
                        // NOTE job can be absent if it is unassigned
                        if let Some(rc) = route_jobs.get_mut(&job) {
                            // NOTE actual insertion context modification via route mut
                            if rc.route.tour.contains(&job) {
                                rc.route_mut().tour.remove(&job);

                                tracker.add_actor(rc.route.actor.clone());
                                tracker.add_job(job.clone());
                            }
                        }
                    });
            }
        });

        tracker.iterate_removed_jobs(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

fn get_routes_cost_savings(insertion_ctx: &InsertionContext) -> Vec<(RouteContext, Vec<(Job, Cost)>)> {
    parallel_collect(&insertion_ctx.solution.routes, |rc| {
        let actor = rc.route.actor.as_ref();
        let mut savings: Vec<(Job, Cost)> = rc
            .route
            .tour
            .all_activities()
            .as_slice()
            .windows(3)
            .fold(HashMap::<Job, Cost>::default(), |mut acc, iter| match iter {
                [start, eval, end] => {
                    let savings = get_cost_savings(actor, start, eval, end, &insertion_ctx.problem.transport);
                    let job = eval.retrieve_job().unwrap_or_else(|| panic!("Unexpected activity without job"));
                    *acc.entry(job).or_insert(0.) += savings;

                    acc
                }
                _ => panic!("Unexpected activity window"),
            })
            .drain()
            .collect();
        savings.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(Less));

        (rc.clone(), savings)
    })
}

fn get_cost_savings(
    actor: &Actor,
    start: &Activity,
    middle: &Activity,
    end: &Activity,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> Cost {
    let waiting_costs = (middle.place.time.start - middle.schedule.arrival).max(0.)
        * (actor.driver.costs.per_waiting_time + actor.vehicle.costs.per_waiting_time);

    let transport_costs = get_cost(actor, start, middle, transport) + get_cost(actor, middle, end, transport)
        - get_cost(actor, start, end, transport);

    waiting_costs + transport_costs
}

#[inline(always)]
fn get_cost(actor: &Actor, from: &Activity, to: &Activity, transport: &Arc<dyn TransportCost + Send + Sync>) -> Cost {
    transport.cost(actor, from.place.location, to.place.location, from.schedule.departure)
}
