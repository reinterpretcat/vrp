#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/ruin/adjusted_string_removal_test.rs"]
mod adjusted_string_removal_test;

use super::Ruin;
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::models::problem::Job;
use crate::models::solution::Tour;
use crate::solver::search::*;
use crate::solver::RefinementContext;
use rosomaxa::prelude::{Float, Random};
use std::cell::RefCell;
use std::sync::Arc;

/// _Adjusted string removal_ ruin strategy based on "Slack Induction by String Removals for
/// Vehicle Routing Problems" by Jan Christiaens, Greet Vanden Berghe.
///
/// Some definitions from the paper:
///  - _string_ is a sequence of consecutive nodes in a tour.
///  - _cardinality_ is the number of customers included in a string or tour.
pub struct AdjustedStringRemoval {
    /// Specifies max removed string cardinality for specific tour.
    lmax: usize,
    /// Specifies average number of removed customers.
    cavg: usize,
    /// Preserved customers' ratio.
    alpha: Float,
    /// Limits.
    limits: RemovalLimits,
}

impl AdjustedStringRemoval {
    /// Creates a new instance of [`AdjustedStringRemoval`].
    pub fn new(lmax: usize, cavg: usize, alpha: Float, limits: RemovalLimits) -> Self {
        Self { lmax, cavg, alpha, limits }
    }

    /// Creates a new instance of [`AdjustedStringRemoval`] with some defaults.
    pub fn new_with_defaults(limits: RemovalLimits) -> Self {
        Self { lmax: 10, cavg: 10, alpha: 0.01, limits }
    }

    /// Calculates initial parameters from paper using 5,6,7 equations.
    fn calculate_limits(&self, routes: &[RouteContext], random: &Arc<dyn Random>) -> (usize, usize) {
        // Equation 5: max removed string cardinality for each tour
        let lsmax = calculate_average_tour_cardinality(routes).min(self.lmax as Float);

        // Equation 6: max number of strings
        let ksmax = 4. * (self.cavg as Float) / (1. + lsmax) - 1.;

        // Equation 7: number of string to be removed
        let ks = random.uniform_real(1., ksmax + 1.).floor() as usize;

        (lsmax as usize, ks)
    }
}

impl Ruin for AdjustedStringRemoval {
    fn run(&self, _: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let problem = insertion_ctx.problem.clone();
        let random = insertion_ctx.environment.random.clone();
        let tracker = RefCell::new(JobRemovalTracker::new(&self.limits, random.as_ref()));
        let mut tabu_list = TabuList::from(&insertion_ctx);

        let (lsmax, ks) = self.calculate_limits(insertion_ctx.solution.routes.as_slice(), &random);
        let seed = select_seed_job_with_tabu_list(&insertion_ctx, &tabu_list).map(|(profile, _, job)| (profile, job));

        select_neighbors(&problem, seed)
            .filter(|job| !tracker.borrow().is_removed_job(job))
            .take_while(|_| tracker.borrow().get_affected_actors() != ks)
            .for_each(|job| {
                let route_idx = insertion_ctx.solution.routes.iter().position(|route_ctx| {
                    !tracker.borrow().is_affected_actor(&route_ctx.route().actor)
                        && route_ctx.route().tour.index(&job).is_some()
                });

                route_idx.into_iter().for_each(|route_idx| {
                    let route_ctx = insertion_ctx.solution.routes.get(route_idx).expect("invalid index");

                    // Equations 8, 9: calculate cardinality of the string removed from the tour
                    let ltmax = route_ctx.route().tour.job_activity_count().min(lsmax);
                    let lt = random.uniform_real(1.0, ltmax as Float + 1.).floor() as usize;

                    if let Some(index) = route_ctx.route().tour.index(&job) {
                        select_string((&route_ctx.route().tour, index), lt, self.alpha, &random)
                            .collect::<Vec<Job>>()
                            .into_iter()
                            .for_each(|job| {
                                if tracker.borrow_mut().try_remove_job(&mut insertion_ctx.solution, route_idx, &job) {
                                    tabu_list.add_job(job);
                                    tabu_list.add_actor(insertion_ctx.solution.routes[route_idx].route().actor.clone());
                                }
                            });
                    }
                });
            });

        tabu_list.inject(&mut insertion_ctx);

        insertion_ctx
    }
}

type JobIter<'a> = Box<dyn Iterator<Item = Job> + 'a>;

/// Calculates average tour cardinality rounded to nearest integral value.
fn calculate_average_tour_cardinality(routes: &[RouteContext]) -> Float {
    (routes.iter().map(|route_ctx| route_ctx.route().tour.job_activity_count() as Float).sum::<Float>()
        / (routes.len() as Float))
        .round()
}

/// Selects string for selected job.
fn select_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    alpha: Float,
    random: &Arc<dyn Random>,
) -> JobIter<'a> {
    if random.is_head_not_tails() {
        sequential_string(seed_tour, cardinality, random)
    } else {
        preserved_string(seed_tour, cardinality, alpha, random)
    }
}

/// Selects sequential string.
fn sequential_string<'a>(seed_tour: (&'a Tour, usize), cardinality: usize, random: &Arc<dyn Random>) -> JobIter<'a> {
    let (begin, end) = lower_bounds(cardinality, seed_tour.0.job_activity_count(), seed_tour.1);
    let start = random.uniform_int(begin as i32, end as i32) as usize;

    Box::new((start..(start + cardinality)).filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())))
}

/// Selects string with preserved jobs.
fn preserved_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    alpha: Float,
    random: &Arc<dyn Random>,
) -> JobIter<'a> {
    let size = seed_tour.0.job_activity_count();
    let index = seed_tour.1;

    let split_size = preserved_cardinality(cardinality, size, alpha, random);
    let total = cardinality + split_size;

    let (begin, end) = lower_bounds(total, size, index);
    let start_total = random.uniform_int(begin as i32, end as i32) as usize;

    let split_start = random.uniform_int(start_total as i32, (start_total + cardinality - 1) as i32) as usize;
    let split_end = split_start + split_size;

    // NOTE if selected job is in split range we should remove it anyway,
    // this line makes sure that string cardinality is kept as requested.
    let total = total - usize::from(index >= split_start && index < split_end);

    Box::new(
        (start_total..(start_total + total))
            .filter(move |&i| i < split_start || i >= split_end || i == index)
            .filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())),
    )
}

/// Returns range of possible lower bounds.
#[allow(clippy::manual_clamp)]
fn lower_bounds(string_crd: usize, tour_crd: usize, index: usize) -> (usize, usize) {
    let string_crd = string_crd as i32;
    let tour_crd = tour_crd as i32;
    let index = index as i32;

    let start = (index - string_crd).max(1);
    let end = (index + string_crd).min(tour_crd - string_crd).max(start);

    (start as usize, end as usize)
}

/// Calculates preserved substring cardinality.
fn preserved_cardinality(string_crd: usize, tour_crd: usize, alpha: Float, random: &Arc<dyn Random>) -> usize {
    if string_crd == tour_crd {
        return 0;
    }

    let mut preserved_crd = 1_usize;
    while string_crd + preserved_crd < tour_crd {
        if random.is_hit(alpha) {
            break;
        } else {
            preserved_crd += 1
        }
    }
    preserved_crd
}
