use super::super::super::rand::prelude::SliceRandom;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::LocalSearch;
use crate::solver::RefinementContext;

/// A local search operator which tries to exchange jobs in random way inside one route.
pub struct ExchangeIntraRouteRandom {
    _probability: f64,
    _noise_range: (f64, f64),
}

impl ExchangeIntraRouteRandom {
    /// Creates a new instance of `ExchangeIntraRouteRandom`.
    pub fn new(probability: f64, min: f64, max: f64) -> Self {
        Self { _probability: probability, _noise_range: (min, max) }
    }
}

impl Default for ExchangeIntraRouteRandom {
    fn default() -> Self {
        Self::new(1., 0.5, 2.)
    }
}

impl LocalSearch for ExchangeIntraRouteRandom {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if !insertion_ctx.solution.required.is_empty() {
            return None;
        }

        if let Some(route_idx) = get_random_route_idx(insertion_ctx) {
            let mut new_insertion_ctx = insertion_ctx.deep_copy();
            let route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

            let jobs = get_shuffled_jobs(insertion_ctx, route_ctx).into_iter().take(2).collect::<Vec<_>>();
            if jobs.len() < 2 {
                return None;
            }

            jobs.iter().for_each(|job| {
                assert!(route_ctx.route_mut().tour.remove(&job));
            });
            new_insertion_ctx.solution.required.extend(jobs.into_iter());

            // TODO cannot use old approach due to changing route count dynamically
            None
        } else {
            None
        }
    }
}

fn get_shuffled_jobs(insertion_ctx: &InsertionContext, route_ctx: &RouteContext) -> Vec<Job> {
    let mut jobs =
        route_ctx.route.tour.jobs().filter(|job| !insertion_ctx.solution.locked.contains(job)).collect::<Vec<_>>();
    jobs.shuffle(&mut insertion_ctx.random.get_rng());

    jobs
}

fn get_random_route_idx(insertion_ctx: &InsertionContext) -> Option<usize> {
    let routes = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter_map(|(idx, rc)| if rc.route.tour.job_count() > 1 { Some(idx) } else { None })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        None
    } else {
        Some(routes[insertion_ctx.random.uniform_int(0, (routes.len() - 1) as i32) as usize])
    }
}
