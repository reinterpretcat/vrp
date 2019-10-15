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
}

impl Default for AdjustedStringRemoval {
    fn default() -> Self {
        Self::new(10, 10, 0.01)
    }
}

impl RuinStrategy for AdjustedStringRemoval {
    fn ruin_solution(refinement_ctx: &RefinementContext, solution: &Solution) -> InsertionContext {
        let jobs: HashSet<Arc<Job>> = HashSet::new();
        let routes: HashSet<Box<Route>> = HashSet::new();
        let insertion_cxt = create_insertion_context(refinement_ctx, solution);

        select_string(&refinement_ctx.problem, solution, &insertion_cxt.random)
            .filter(|job| !jobs.contains(job) && !solution.unassigned.contains_key(job))
            .for_each(|job| {
                // TODO
            });

        unimplemented!()
    }
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
