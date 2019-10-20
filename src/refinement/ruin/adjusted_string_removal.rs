#[cfg(test)]
#[path = "../../../tests/unit/refinement/ruin/adjusted_string_removal_test.rs"]
mod adjusted_string_removal_test;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::construction::states::{InsertionContext, RouteContext};
use crate::models::problem::Job;
use crate::models::solution::{Actor, Route, Tour};
use crate::models::{Problem, Solution};
use crate::refinement::ruin::Ruin;
use crate::refinement::RefinementContext;
use crate::utils::Random;
use std::borrow::Borrow;
use std::iter::{empty, once};
use std::ops::Deref;

/// "Adjusted string removal" strategy based on "Slack Induction by String Removals for
/// Vehicle Routing Problems" (aka SISR) by Jan Christiaens, Greet Vanden Berghe.
/// Some definitions from the paper:
///     String is a sequence of consecutive nodes in a tour.
///     Cardinality is the number of customers included in a string or tour.
pub struct AdjustedStringRemoval {
    /// Specifies max removed string cardinality for specific tour.
    lmax: usize,
    /// Specifies average number of removed customers.
    cavg: usize,
    /// Preserved customers ratio.
    alpha: f64,
}

impl AdjustedStringRemoval {
    fn new(lmax: usize, cavg: usize, alpha: f64) -> Self {
        Self { lmax, cavg, alpha }
    }

    /// Calculates initial parameters from paper using 5,6,7 equations.
    fn calculate_limits(&self, routes: &Vec<RouteContext>, random: &Arc<dyn Random + Send + Sync>) -> (usize, usize) {
        // Equation 5: max removed string cardinality for each tour
        let lsmax = calculate_average_tour_cardinality(routes).min(self.lmax as f64);

        // Equation 6: max number of strings
        let ksmax = 4. * (self.cavg as f64) / (1. + lsmax) - 1.;

        // Equation 7: number of string to be removed
        let ks = random.uniform_real(1., ksmax + 1.).floor() as usize;

        (lsmax as usize, ks)
    }
}

impl Default for AdjustedStringRemoval {
    fn default() -> Self {
        Self::new(10, 10, 0.01)
    }
}

impl Ruin for AdjustedStringRemoval {
    fn run(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;
        let mut jobs: RwLock<HashSet<Arc<Job>>> = RwLock::new(HashSet::new());
        let mut actors: RwLock<HashSet<Arc<Actor>>> = RwLock::new(HashSet::new());
        let routes: Vec<RouteContext> = insertion_ctx.solution.routes.clone();

        let (lsmax, ks) = self.calculate_limits(&routes, &insertion_ctx.random);

        select_seed_jobs(&insertion_ctx.problem, &routes, &insertion_ctx.random)
            .filter(|job| !jobs.read().unwrap().contains(job))
            .take_while(|_| actors.read().unwrap().len() != ks)
            .for_each(|job| {
                insertion_ctx
                    .solution
                    .routes
                    .iter()
                    .filter(|rc| {
                        let route = rc.route.read().unwrap();
                        !actors.read().unwrap().contains(&route.actor) && route.tour.index(&job).is_some()
                    })
                    .for_each(|rc| {
                        let mut route = rc.route.write().unwrap();

                        // Equations 8, 9: calculate cardinality of the string removed from the tour
                        let ltmax = route.tour.activity_count().min(lsmax);
                        let lt = insertion_ctx.random.uniform_real(1.0, ltmax as f64 + 1.).floor() as usize;

                        if let Some(index) = route.tour.index(&job) {
                            actors.write().unwrap().insert(route.actor.clone());
                            select_string((&route.tour, index), lt, self.alpha, &insertion_ctx.random)
                                .filter(|job| !insertion_ctx.locked.contains(job))
                                .collect::<Vec<Arc<Job>>>()
                                .into_iter()
                                .for_each(|job| {
                                    route.tour.remove(&job);
                                    jobs.write().unwrap().insert(job);
                                });
                        }
                    });
            });

        jobs.write().unwrap().iter().for_each(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx.remove_empty_routes();

        insertion_ctx
    }
}

type JobIter<'a> = Box<dyn Iterator<Item = Arc<Job>> + 'a>;

/// Calculates average tour cardinality rounded to nearest integral value.
fn calculate_average_tour_cardinality(routes: &Vec<RouteContext>) -> f64 {
    (routes.iter().fold(0., |acc, rc| acc + rc.route.read().unwrap().tour.activity_count() as f64)
        / routes.len() as f64)
        .round()
}

/// Selects string for selected job.
fn select_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    alpha: f64,
    random: &Arc<dyn Random + Send + Sync>,
) -> JobIter<'a> {
    if random.is_head_not_tails() {
        sequential_string(seed_tour, cardinality, random)
    } else {
        preserved_string(seed_tour, cardinality, alpha, random)
    }
}

/// Selects sequential string.
fn sequential_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    random: &Arc<dyn Random + Send + Sync>,
) -> JobIter<'a> {
    let (begin, end) = lower_bounds(cardinality, seed_tour.0.activity_count(), seed_tour.1);
    let start = random.uniform_int(begin as i32, end as i32) as usize;

    Box::new(
        (start..(start + cardinality)).rev().filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())),
    )
}

/// Selects string with preserved jobs.
fn preserved_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    alpha: f64,
    random: &Arc<dyn Random + Send + Sync>,
) -> JobIter<'a> {
    let size = seed_tour.0.activity_count();
    let index = seed_tour.1;

    let split = preserved_cardinality(cardinality, size, alpha, random);
    let mut total = cardinality + split;

    let (begin, end) = lower_bounds(total, size, index);
    let start_total = random.uniform_int(begin as i32, end as i32) as usize;

    let split_start = random.uniform_int(start_total as i32, (start_total + cardinality - 1) as i32) as usize;
    let split_end = split_start + split;

    // NOTE if selected job is in split range we should remove it anyway,
    // this line makes sure that string cardinality is kept as requested.
    total -= if index >= split_start && index < split_end { 1 } else { 0 };

    Box::new(
        (start_total..(start_total + total))
            .rev()
            .filter(move |&i| i < split_start || i >= split_end || i == index)
            .filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())),
    )
}

/// Returns randomly selected job within all its neighbours.
fn select_seed_jobs<'a>(
    problem: &'a Problem,
    routes: &'a Vec<RouteContext>,
    random: &Arc<dyn Random + Send + Sync>,
) -> JobIter<'a> {
    let seed = select_seed_job(routes, random);

    if let Some((rc, job)) = seed {
        return Box::new(once(job.clone()).chain(problem.jobs.neighbors(
            rc.route.read().unwrap().actor.vehicle.profile,
            &job,
            Default::default(),
            std::f64::MAX,
        )));
    }

    Box::new(empty())
}

/// Selects seed job from existing solution
fn select_seed_job<'a>(
    routes: &'a Vec<RouteContext>,
    random: &Arc<dyn Random + Send + Sync>,
) -> Option<(&'a RouteContext, Arc<Job>)> {
    if routes.is_empty() {
        return None;
    }

    let route_index = random.uniform_int(0, (routes.len() - 1) as i32) as usize;
    let mut ri = route_index;

    loop {
        let rc = routes.get(ri).unwrap();

        if rc.route.read().unwrap().tour.has_jobs() {
            let job = select_random_job(rc, random);
            if let Some(job) = job {
                return Some((rc, job));
            }
        }

        ri = (ri + 1) % routes.len();
        if ri == route_index {
            break;
        }
    }

    None
}

fn select_random_job(rc: &RouteContext, random: &Arc<dyn Random + Send + Sync>) -> Option<Arc<Job>> {
    let route = rc.route.read().unwrap();
    let size = route.tour.activity_count();
    if size == 0 {
        return None;
    }

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = route.tour.get(ai).and_then(|a| a.retrieve_job());

        if job.is_some() {
            return job;
        }

        ai = (ai + 1) % (size + 1);
        if ai == activity_index {
            break;
        }
    }

    None
}

/// Returns range of possible lower bounds.
fn lower_bounds(string_crd: usize, tour_crd: usize, index: usize) -> (usize, usize) {
    let string_crd = string_crd as i32;
    let tour_crd = tour_crd as i32;
    let index = index as i32;

    let start = (index - string_crd + 1).max(1);
    let end = (tour_crd - string_crd + 1).min(start + string_crd);

    (start as usize, end as usize)
}

/// Calculates preserved substring cardinality.
fn preserved_cardinality(
    string_crd: usize,
    tour_crd: usize,
    alpha: f64,
    random: &Arc<dyn Random + Send + Sync>,
) -> usize {
    if string_crd == tour_crd {
        return 0;
    }

    let mut preserved_crd = 1usize;
    while string_crd + preserved_crd < tour_crd {
        if random.uniform_real(0.0, 1.0) < alpha {
            break;
        } else {
            preserved_crd = preserved_crd + 1
        }
    }
    preserved_crd
}
