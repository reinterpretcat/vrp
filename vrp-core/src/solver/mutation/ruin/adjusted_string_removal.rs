#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/ruin/adjusted_string_removal_test.rs"]
mod adjusted_string_removal_test;

use std::sync::Arc;

use super::Ruin;
use crate::construction::heuristics::{InsertionContext, RouteContext};
use crate::models::problem::Job;
use crate::models::solution::Tour;
use crate::solver::mutation::{select_seed_jobs, RuinLimits};
use crate::solver::RefinementContext;
use rosomaxa::utils::Random;

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
    /// Preserved customers ratio.
    alpha: f64,
}

impl AdjustedStringRemoval {
    /// Creates a new instance of [`AdjustedStringRemoval`].
    pub fn new(lmax: usize, cavg: usize, alpha: f64) -> Self {
        Self { lmax, cavg, alpha }
    }

    /// Calculates initial parameters from paper using 5,6,7 equations.
    fn calculate_limits(&self, routes: &[RouteContext], random: &Arc<dyn Random + Send + Sync>) -> (usize, usize) {
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
    fn run(&self, _refinement_ctx: &RefinementContext, mut insertion_ctx: InsertionContext) -> InsertionContext {
        let routes: Vec<RouteContext> = insertion_ctx.solution.routes.clone();
        let problem = insertion_ctx.problem.clone();
        let locked = insertion_ctx.solution.locked.clone();
        let random = insertion_ctx.environment.random.clone();

        let (lsmax, ks) = self.calculate_limits(&routes, &random);

        let limits = RuinLimits::default();
        let tracker = limits.get_tracker();

        select_seed_jobs(&problem, &routes, &random)
            .filter(|job| !tracker.is_removed_job(job))
            .take_while(|_| tracker.get_affected_actors() != ks)
            .for_each(|job| {
                insertion_ctx
                    .solution
                    .routes
                    .iter_mut()
                    .find(|rc| !tracker.is_affected_actor(&rc.route.actor) && rc.route.tour.index(&job).is_some())
                    .iter_mut()
                    .for_each(|rc| {
                        // Equations 8, 9: calculate cardinality of the string removed from the tour
                        let ltmax = rc.route.tour.job_activity_count().min(lsmax);
                        let lt = random.uniform_real(1.0, ltmax as f64 + 1.).floor() as usize;

                        if let Some(index) = rc.route.tour.index(&job) {
                            tracker.add_actor(rc.route.actor.clone());
                            select_string((&rc.route.tour, index), lt, self.alpha, &random)
                                .filter(|job| !locked.contains(job))
                                .collect::<Vec<Job>>()
                                .into_iter()
                                .for_each(|job| {
                                    rc.route_mut().tour.remove(&job);
                                    tracker.add_job(job);
                                });
                        }
                    });
            });

        tracker.iterate_removed_jobs(|job| insertion_ctx.solution.required.push(job.clone()));

        insertion_ctx
    }
}

type JobIter<'a> = Box<dyn Iterator<Item = Job> + 'a>;

/// Calculates average tour cardinality rounded to nearest integral value.
fn calculate_average_tour_cardinality(routes: &[RouteContext]) -> f64 {
    (routes.iter().map(|rc| rc.route.tour.job_activity_count() as f64).sum::<f64>() / (routes.len() as f64)).round()
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
    let (begin, end) = lower_bounds(cardinality, seed_tour.0.job_activity_count(), seed_tour.1);
    let start = random.uniform_int(begin as i32, end as i32) as usize;

    Box::new((start..(start + cardinality)).filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())))
}

/// Selects string with preserved jobs.
fn preserved_string<'a>(
    seed_tour: (&'a Tour, usize),
    cardinality: usize,
    alpha: f64,
    random: &Arc<dyn Random + Send + Sync>,
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
    let total = total - if index >= split_start && index < split_end { 1 } else { 0 };

    Box::new(
        (start_total..(start_total + total))
            .filter(move |&i| i < split_start || i >= split_end || i == index)
            .filter_map(move |i| seed_tour.0.get(i).and_then(|a| a.retrieve_job())),
    )
}

/// Returns range of possible lower bounds.
fn lower_bounds(string_crd: usize, tour_crd: usize, index: usize) -> (usize, usize) {
    let string_crd = string_crd as i32;
    let tour_crd = tour_crd as i32;
    let index = index as i32;

    let start = (index - string_crd).max(1);
    let end = (index + string_crd).min(tour_crd - string_crd).max(start);

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
