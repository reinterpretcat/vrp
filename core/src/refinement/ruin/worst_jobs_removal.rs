#[cfg(test)]
#[path = "../../../tests/unit/refinement/ruin/worst_jobs_removal_test.rs"]
mod worst_jobs_removal_test;

extern crate rand;
extern crate rayon;

use crate::construction::states::{InsertionContext, RouteContext, SolutionContext};
use crate::models::common::Cost;
use crate::models::problem::{Actor, Job, TransportCost};
use crate::models::solution::TourActivity;
use crate::refinement::ruin::Ruin;
use crate::refinement::RefinementContext;
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use rayon::prelude::*;
use std::cmp::Ordering::Less;
use std::iter::once;
use std::sync::{Arc, RwLock};

/// Detects the most cost expensive jobs in each route and delete them with their neighbours
pub struct WorstJobRemoval {
    threshold: usize,
    range: (i32, i32),
}

impl Default for WorstJobRemoval {
    fn default() -> Self {
        Self::new(30, 1, 8)
    }
}

impl Ruin for WorstJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        let problem = insertion_ctx.problem.clone();
        let locked = insertion_ctx.locked.clone();
        let random = insertion_ctx.random.clone();

        let mut route_jobs = get_route_jobs(&insertion_ctx.solution);
        let mut routes_savings = get_routes_cost_savings(&insertion_ctx);
        let removed_jobs: RwLock<HashSet<Arc<Job>>> = RwLock::new(HashSet::default());

        routes_savings.shuffle(&mut rand::thread_rng());

        routes_savings.iter().take_while(|_| removed_jobs.read().unwrap().len() <= self.threshold).for_each(
            |(rc, savings)| {
                let worst = savings.iter().filter(|(job, _)| !locked.contains(job)).next();
                if let Some((job, _)) = worst {
                    let remove = random.uniform_int(self.range.0, self.range.1) as usize;
                    once(job.clone())
                        .chain(problem.jobs.neighbors(
                            rc.route.actor.vehicle.profile,
                            &job,
                            Default::default(),
                            std::f64::MAX,
                        ))
                        .filter(|job| !locked.contains(job))
                        .take(remove)
                        .for_each(|job| {
                            // NOTE job can be absent if it is unassigned
                            if let Some(rc) = route_jobs.get_mut(&job) {
                                // NOTE actual insertion context modification via route mut
                                rc.route_mut().tour.remove(&job);
                                removed_jobs.write().unwrap().insert(job);
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
    pub fn new(threshold: usize, min: usize, max: usize) -> Self {
        assert!(min <= max);

        Self { threshold, range: (min as i32, max as i32) }
    }
}

fn get_route_jobs(solution: &SolutionContext) -> HashMap<Arc<Job>, RouteContext> {
    solution.routes.iter().fold(HashMap::default(), |acc, rc| {
        rc.route.tour.jobs().fold(acc, |mut acc, job| {
            acc.insert(job, rc.clone());
            acc
        })
    })
}

fn get_routes_cost_savings(insertion_ctx: &InsertionContext) -> Vec<(RouteContext, Vec<(Arc<Job>, Cost)>)> {
    insertion_ctx
        .solution
        .routes
        .par_iter()
        .map(|rc| {
            let actor = rc.route.actor.as_ref();
            let mut savings: Vec<(Arc<Job>, Cost)> = rc
                .route
                .tour
                .all_activities()
                .as_slice()
                .windows(3)
                .fold(HashMap::<Arc<Job>, Cost>::default(), |mut acc, iter| match iter {
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
        .collect()
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
