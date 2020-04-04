#[cfg(test)]
#[path = "../../../../tests/unit/refinement/mutation/ruin/worst_jobs_removal_test.rs"]
mod worst_jobs_removal_test;

extern crate rand;

use super::Ruin;
use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::{Actor, Job, TransportCost};
use crate::models::solution::TourActivity;
use crate::refinement::RefinementContext;
use crate::utils::parallel_collect;
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use std::cmp::Ordering::Less;
use std::iter::once;
use std::sync::{Arc, RwLock};

/// A ruin strategy which detects the most cost expensive jobs in each route and delete them
/// with their neighbours.
pub struct WorstJobRemoval {
    threshold: usize,
    worst_skip: i32,
    range: (i32, i32),
}

impl Default for WorstJobRemoval {
    fn default() -> Self {
        Self::new(32, 4, (1, 4))
    }
}

impl Ruin for WorstJobRemoval {
    fn run(&self, _refinement_ctx: &mut RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.random.clone();

        let can_remove_job = |job: &Job| -> bool {
            let solution = &insertion_ctx.solution;
            !solution.locked.contains(job) && !solution.unassigned.contains_key(job)
        };

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let mut routes_savings = get_routes_cost_savings(&insertion_ctx);
        let removed_jobs: RwLock<HashSet<Job>> = RwLock::new(HashSet::default());

        routes_savings.shuffle(&mut rand::thread_rng());

        routes_savings.iter().take_while(|_| removed_jobs.read().unwrap().len() <= self.threshold).for_each(
            |(rc, savings)| {
                let skip = savings.len().min(random.uniform_int(0, self.worst_skip) as usize);
                let worst = savings.iter().filter(|(job, _)| can_remove_job(job)).skip(skip).next();

                if let Some((job, _)) = worst {
                    let remove = random.uniform_int(self.range.0, self.range.1) as usize;
                    once(job.clone())
                        .chain(problem.jobs.neighbors(
                            rc.route.actor.vehicle.profile,
                            &job,
                            Default::default(),
                            std::f64::MAX,
                        ))
                        .filter(|job| can_remove_job(job))
                        .take(remove)
                        .for_each(|job| {
                            // NOTE job can be absent if it is unassigned
                            if let Some(rc) = route_jobs.get_mut(&job) {
                                // NOTE actual insertion context modification via route mut
                                if rc.route_mut().tour.remove(&job) {
                                    removed_jobs.write().unwrap().insert(job);
                                }
                            }
                        });
                }
            },
        );

        removed_jobs.write().unwrap().iter().for_each(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

impl WorstJobRemoval {
    pub fn new(threshold: usize, worst_skip: usize, range: (usize, usize)) -> Self {
        assert!(range.0 <= range.1);

        Self { threshold, worst_skip: worst_skip as i32, range: (range.0 as i32, range.1 as i32) }
    }
}

fn get_route_jobs(solution: &SolutionContext) -> HashMap<Job, RouteContext> {
    solution
        .routes
        .iter()
        .flat_map(|rc| rc.route.tour.jobs().collect::<Vec<_>>().into_iter().map(move |job| (job, rc.clone())))
        .collect()
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

#[inline(always)]
fn get_cost_savings(
    actor: &Actor,
    start: &TourActivity,
    middle: &TourActivity,
    end: &TourActivity,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> Cost {
    get_cost(actor, start, middle, transport) + get_cost(actor, middle, end, transport)
        - get_cost(actor, start, end, transport)
}

#[inline(always)]
fn get_cost(
    actor: &Actor,
    from: &TourActivity,
    to: &TourActivity,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> Cost {
    transport.cost(actor, from.place.location, to.place.location, from.schedule.departure)
}
