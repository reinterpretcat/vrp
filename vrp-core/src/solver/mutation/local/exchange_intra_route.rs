use super::super::super::rand::prelude::SliceRandom;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::LocalSearch;
use crate::solver::RefinementContext;
use crate::utils::{Noise, Random};

/// A local search operator which tries to exchange jobs in random way inside one route.
pub struct ExchangeIntraRouteRandom {
    noise_range: (f64, f64),
}

impl ExchangeIntraRouteRandom {
    /// Creates a new instance of `ExchangeIntraRouteRandom`.
    pub fn new(noise_range: (f64, f64)) -> Self {
        Self { noise_range }
    }
}

impl Default for ExchangeIntraRouteRandom {
    fn default() -> Self {
        Self::new((0.5, 2.))
    }
}

impl LocalSearch for ExchangeIntraRouteRandom {
    fn explore(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        if !insertion_ctx.solution.required.is_empty() {
            return None;
        }

        if let Some(route_idx) = get_random_route_idx(insertion_ctx) {
            let mut new_insertion_ctx = insertion_ctx.deep_copy();
            let route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

            let jobs =
                get_shuffled_jobs(route_ctx, insertion_ctx.random.as_ref()).into_iter().take(2).collect::<Vec<_>>();
            jobs.iter().for_each(|job| {
                assert!(route_ctx.route_mut().tour.remove(&job));
            });
            new_insertion_ctx.solution.required.extend(jobs.into_iter());

            // NOTE set probability to 1 to improve chances for different route configuration
            let noise = Noise::new(1., self.noise_range, insertion_ctx.random.clone());
            new_insertion_ctx = InsertionHeuristic::default().process(
                &AllJobSelector::default(),
                &PairJobMapReducer::new(
                    Box::new(SpecificRouteSelector { route_idx }),
                    Box::new(NoiseResultSelector::new(noise)),
                ),
                new_insertion_ctx,
                &refinement_ctx.quota,
            );

            Some(new_insertion_ctx)
        } else {
            None
        }
    }
}

struct SpecificRouteSelector {
    route_idx: usize,
}

impl RouteSelector for SpecificRouteSelector {
    fn select<'a>(
        &'a self,
        insertion_ctx: &'a InsertionContext,
        _: &'a Job,
    ) -> Box<dyn Iterator<Item = RouteContext> + 'a> {
        Box::new(insertion_ctx.solution.routes.iter().skip(self.route_idx).take(1).cloned())
    }
}

fn get_shuffled_jobs(route_ctx: &RouteContext, random: &(dyn Random + Sync + Send)) -> Vec<Job> {
    let mut jobs = route_ctx.route.tour.jobs().collect::<Vec<_>>();
    jobs.shuffle(&mut random.get_rng());

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
