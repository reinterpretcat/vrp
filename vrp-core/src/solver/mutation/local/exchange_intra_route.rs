use super::super::super::rand::prelude::SliceRandom;
use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::LocalOperator;
use crate::solver::RefinementContext;
use crate::utils::Noise;

/// A local search operator which tries to exchange jobs in random way inside one route.
pub struct ExchangeIntraRouteRandom {
    probability: f64,
    noise_range: (f64, f64),
}

impl ExchangeIntraRouteRandom {
    /// Creates a new instance of `ExchangeIntraRouteRandom`.
    pub fn new(probability: f64, min: f64, max: f64) -> Self {
        Self { probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeIntraRouteRandom {
    fn default() -> Self {
        Self::new(0.05, 0.75, 1.25)
    }
}

impl LocalOperator for ExchangeIntraRouteRandom {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if !insertion_ctx.solution.required.is_empty() {
            return None;
        }

        if let Some(route_idx) = get_random_route_idx(insertion_ctx) {
            let mut new_insertion_ctx = insertion_ctx.deep_copy();
            let mut route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

            if let Some(job) = get_shuffled_jobs(insertion_ctx, route_ctx).into_iter().next() {
                assert!(route_ctx.route_mut().tour.remove(&job));
                new_insertion_ctx.problem.constraint.accept_route_state(&mut route_ctx);

                let insertion = evaluate_job_insertion_in_route(
                    &insertion_ctx,
                    &route_ctx,
                    &job,
                    InsertionPosition::Any,
                    InsertionResult::make_failure(),
                    &NoiseResultSelector::new(Noise::new(
                        self.probability,
                        self.noise_range,
                        insertion_ctx.environment.random.clone(),
                    )),
                );

                return match &insertion {
                    InsertionResult::Success(_) => {
                        apply_insertion_result(&mut new_insertion_ctx, insertion);
                        finalize_insertion_ctx(&mut new_insertion_ctx);
                        Some(new_insertion_ctx)
                    }
                    _ => None,
                };
            }
        }

        None
    }
}

fn get_shuffled_jobs(insertion_ctx: &InsertionContext, route_ctx: &RouteContext) -> Vec<Job> {
    let mut jobs =
        route_ctx.route.tour.jobs().filter(|job| !insertion_ctx.solution.locked.contains(job)).collect::<Vec<_>>();
    jobs.shuffle(&mut insertion_ctx.environment.random.get_rng());

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
        Some(routes[insertion_ctx.environment.random.uniform_int(0, (routes.len() - 1) as i32) as usize])
    }
}
