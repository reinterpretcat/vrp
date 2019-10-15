use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::construction::states::InsertionContext;
use crate::models::problem::Job;
use crate::models::solution::Route;
use crate::models::{Problem, Solution};
use crate::refinement::ruin::{create_insertion_context, RuinStrategy};
use crate::refinement::RefinementContext;
use crate::utils::Random;
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
    fn calculate_limits(&self, solution: &Solution, random: &Arc<dyn Random + Send + Sync>) -> (usize, usize) {
        // Equation 5: max removed string cardinality for each tour
        let lsmax = calculate_average_tour_cardinality(solution).min(self.lmax as f64);

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

impl RuinStrategy for AdjustedStringRemoval {
    fn ruin_solution(&self, refinement_ctx: &RefinementContext, solution: &Solution) -> InsertionContext {
        let jobs: HashSet<Arc<Job>> = HashSet::new();
        let routes: HashSet<Box<Route>> = HashSet::new();
        let insertion_cxt = create_insertion_context(refinement_ctx, solution);
        let (lsmax, ks) = self.calculate_limits(solution, &insertion_cxt.random);

        select_string(&refinement_ctx.problem, solution, &insertion_cxt.random)
            .filter(|job| !jobs.contains(job) && !solution.unassigned.contains_key(job))
            .for_each(|job| {
                insertion_cxt
                    .solution
                    .routes
                    .iter()
                    .filter(|rc| {
                        let route = rc.route.read().unwrap();
                        !routes.contains(route.deref()) && route.tour.index(&job).is_none()
                    })
                    .for_each(|rc| {
                        // Equations 8, 9: calculate cardinality of the string removed from the tour
                        let ltmax = rc.route.read().unwrap().tour.job_count().min(lsmax);
                        let lt = insertion_cxt.random.uniform_real(1.0, ltmax as f64 + 1.).floor() as usize;
                    });
            });

        unimplemented!()
    }
}

/// Calculates average tour cardinality rounded to nearest integral value.
fn calculate_average_tour_cardinality(solution: &Solution) -> f64 {
    (solution.routes.iter().fold(0., |acc, route| acc + route.tour.job_count() as f64) / solution.routes.len() as f64)
        .round()
}

/// Returns randomly selected job within all its neighbours.
fn select_string<'a>(
    problem: &'a Problem,
    solution: &'a Solution,
    random: &Arc<dyn Random + Send + Sync>,
) -> Box<dyn Iterator<Item = Arc<Job>> + 'a> {
    let seed = select_seed_job(&solution.routes, random);

    if let Some((route, job)) = seed {
        return Box::new(once(job.clone()).chain(problem.jobs.neighbors(
            route.actor.vehicle.profile,
            &job,
            Default::default(),
            std::f64::MAX,
        )));
    }

    Box::new(empty())
}

/// Selects seed job from existing solution
fn select_seed_job<'a>(
    routes: &'a Vec<Route>,
    random: &Arc<dyn Random + Send + Sync>,
) -> Option<(&'a Route, Arc<Job>)> {
    if routes.is_empty() {
        return None;
    }

    let route_index = random.uniform_int(0, routes.len() as i32) as usize;
    let mut ri = route_index;

    loop {
        let route = routes.get(ri).unwrap();

        if route.tour.has_jobs() {
            let job = select_random_job(route, random);
            if let Some(job) = job {
                return Some((route, job));
            }
        }

        ri = (ri + 1) % routes.len();
        if ri == route_index {
            break;
        }
    }

    None
}

fn select_random_job(route: &Route, random: &Arc<dyn Random + Send + Sync>) -> Option<Arc<Job>> {
    let size = route.tour.activity_count();
    if size == 0 {
        return None;
    }
    let size = size + 1;

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = route.tour.get(ai).and_then(|a| a.retrieve_job());

        if job.is_some() {
            return job;
        }

        ai = (ai + 1) % size;
        if ai == activity_index {
            break;
        }
    }

    None
}
