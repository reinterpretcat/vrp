use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::RefinementContext;
use crate::solver::search::LocalOperator;
use crate::utils::Noise;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::{Float, HeuristicSolution};

/// A local search operator which tries to exchange jobs in random way inside one route.
pub struct ExchangeIntraRouteRandom {
    probability: Float,
    noise_range: (Float, Float),
}

impl ExchangeIntraRouteRandom {
    /// Creates a new instance of `ExchangeIntraRouteRandom`.
    pub fn new(probability: Float, min: Float, max: Float) -> Self {
        Self { probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeIntraRouteRandom {
    fn default() -> Self {
        Self::new(0.05, -0.25, 0.25)
    }
}

impl LocalOperator for ExchangeIntraRouteRandom {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if let Some(route_idx) = get_random_route_idx(insertion_ctx) {
            let random = insertion_ctx.environment.random.clone();
            let route_ctx = insertion_ctx.solution.routes.get(route_idx).unwrap();

            if let Some(job) = get_shuffled_jobs(insertion_ctx, route_ctx).into_iter().next() {
                let mut new_insertion_ctx = insertion_ctx.deep_copy();
                let new_route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

                assert!(new_route_ctx.route_mut().tour.remove(&job));
                new_insertion_ctx.solution.required.push(job.clone());
                new_insertion_ctx.problem.goal.accept_route_state(new_route_ctx);

                let leg_selection = LegSelection::Stochastic(random.clone());
                let result_selector = NoiseResultSelector::new(Noise::new_with_addition(
                    self.probability,
                    self.noise_range,
                    random.clone(),
                ));
                let eval_ctx = EvaluationContext {
                    goal: &insertion_ctx.problem.goal,
                    job: &job,
                    leg_selection: &leg_selection,
                    result_selector: &result_selector,
                };

                let new_route_ctx = new_insertion_ctx.solution.routes.get(route_idx).unwrap();
                let insertion = eval_job_insertion_in_route(
                    &new_insertion_ctx,
                    &eval_ctx,
                    new_route_ctx,
                    InsertionPosition::Any,
                    InsertionResult::make_failure(),
                );

                return match insertion {
                    InsertionResult::Success(success) => {
                        apply_insertion_success(&mut new_insertion_ctx, success);
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
    let mut jobs = route_ctx
        .route()
        .tour
        .jobs()
        .filter(|job| !insertion_ctx.solution.locked.contains(*job))
        .cloned()
        .collect::<Vec<_>>();
    jobs.shuffle(&mut insertion_ctx.environment.random.get_rng());

    jobs
}

fn get_random_route_idx(insertion_ctx: &InsertionContext) -> Option<usize> {
    let routes = insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter_map(|(idx, route_ctx)| if route_ctx.route().tour.job_count() > 1 { Some(idx) } else { None })
        .collect::<Vec<_>>();

    if routes.is_empty() {
        None
    } else {
        Some(routes[insertion_ctx.environment.random.uniform_int(0, (routes.len() - 1) as i32) as usize])
    }
}
