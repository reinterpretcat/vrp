#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/local/exchange_inter_route_test.rs"]
mod exchange_inter_route_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::{select_seed_job, LocalOperator};
use crate::solver::RefinementContext;
use crate::utils::{map_reduce, Noise};

/// A local search operator which tries to exchange jobs in best way between different routes.
pub struct ExchangeInterRouteBest {
    noise_probability: f64,
    noise_range: (f64, f64),
}

/// A local search operator which tries to exchange random jobs between different routes.
pub struct ExchangeInterRouteRandom {
    noise_probability: f64,
    noise_range: (f64, f64),
}

impl ExchangeInterRouteBest {
    /// Creates a new instance of `ExchangeInterRouteBest`.
    pub fn new(noise_probability: f64, min: f64, max: f64) -> Self {
        Self { noise_probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeInterRouteBest {
    fn default() -> Self {
        Self::new(0.1, 0.9, 1.1)
    }
}

impl LocalOperator for ExchangeInterRouteBest {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        find_best_insertion_pair(
            insertion_ctx,
            Noise::new(self.noise_probability, self.noise_range, insertion_ctx.random.clone()),
            Box::new(|_| true),
            Box::new(|_| true),
        )
    }
}

impl ExchangeInterRouteRandom {
    /// Creates a new instance of `ExchangeInterRouteRandom`.
    pub fn new(noise_probability: f64, min: f64, max: f64) -> Self {
        Self { noise_probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeInterRouteRandom {
    fn default() -> Self {
        Self::new(0.1, 0.9, 1.1)
    }
}

impl LocalOperator for ExchangeInterRouteRandom {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        find_best_insertion_pair(
            insertion_ctx,
            Noise::new(self.noise_probability, self.noise_range, insertion_ctx.random.clone()),
            {
                let random = insertion_ctx.random.clone();
                Box::new(move |_idx| random.is_head_not_tails())
            },
            {
                let random = insertion_ctx.random.clone();
                Box::new(move |_idx| random.is_head_not_tails())
            },
        )
    }
}

type InsertionSuccessPair = (InsertionSuccess, InsertionSuccess);

fn find_best_insertion_pair(
    insertion_ctx: &InsertionContext,
    noise: Noise,
    filter_route_indices: Box<dyn Fn(usize) -> bool + Send + Sync>,
    filter_jobs_indices: Box<dyn Fn(usize) -> bool + Send + Sync>,
) -> Option<InsertionContext> {
    if let Some((seed_route_idx, seed_job)) =
        select_seed_job(insertion_ctx.solution.routes.as_slice(), &insertion_ctx.random)
    {
        let locked = &insertion_ctx.solution.locked;

        // bad luck: cannot move locked job
        if locked.contains(&seed_job) {
            return None;
        }

        let new_insertion_ctx = get_new_insertion_ctx(insertion_ctx, &seed_job, seed_route_idx).unwrap();
        let seed_route = new_insertion_ctx.solution.routes.get(seed_route_idx).unwrap();
        let result_selector = NoiseResultSelector::new(noise.clone());

        let insertion_pair = new_insertion_ctx
            .solution
            .routes
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != seed_route_idx && filter_route_indices(*idx))
            .fold(Option::<InsertionSuccessPair>::None, |acc, (_, test_route)| {
                let new_result = map_reduce(
                    test_route
                        .route
                        .tour
                        .jobs()
                        .enumerate()
                        .filter(|(idx, job)| !locked.contains(&job) && filter_jobs_indices(*idx))
                        .collect::<Vec<_>>()
                        .as_slice(),
                    |(_, test_job)| {
                        // try to insert test job into seed tour
                        let seed_success =
                            test_job_insertion(&new_insertion_ctx, &seed_route, &test_job, &result_selector)?;

                        // try to insert seed job into test route
                        let mut test_route = test_route.deep_copy();
                        test_route.route_mut().tour.remove(test_job);
                        new_insertion_ctx.problem.constraint.accept_route_state(&mut test_route);

                        let test_success =
                            test_job_insertion(&new_insertion_ctx, &test_route, &seed_job, &result_selector)?;

                        Some((seed_success, test_success))
                    },
                    || None,
                    |left, right| reduce_pair_with_noise(left, right, &noise),
                );

                reduce_pair_with_noise(acc, new_result, &noise)
            });

        if let Some(insertion_pair) = insertion_pair {
            let mut new_insertion_ctx = new_insertion_ctx;
            apply_insertion_success(&mut new_insertion_ctx, insertion_pair.0);
            apply_insertion_success(&mut new_insertion_ctx, insertion_pair.1);
            finalize_insertion_ctx(&mut new_insertion_ctx);

            return Some(new_insertion_ctx);
        }
    }

    None
}

fn test_job_insertion(
    insertion_ctx: &InsertionContext,
    route: &RouteContext,
    job: &Job,
    result_selector: &(dyn ResultSelector + Send + Sync),
) -> Option<InsertionSuccess> {
    let insertion = evaluate_job_insertion_in_route(
        job,
        &insertion_ctx,
        &route,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
        result_selector,
    );

    match insertion {
        InsertionResult::Failure(_) => None,
        InsertionResult::Success(success) => Some(success),
    }
}

fn apply_insertion_success(insertion_ctx: &mut InsertionContext, insertion_success: InsertionSuccess) {
    let route_index = insertion_ctx
        .solution
        .routes
        .iter()
        .position(|ctx| ctx.route.actor == insertion_success.context.route.actor)
        .unwrap();

    // NOTE replace existing route context with the different
    insertion_ctx.solution.routes[route_index] =
        RouteContext::new_with_state(insertion_success.context.route.clone(), insertion_success.context.state.clone());

    apply_insertion_result(insertion_ctx, InsertionResult::Success(insertion_success))
}

fn reduce_pair_with_noise(
    left_result: Option<InsertionSuccessPair>,
    right_result: Option<InsertionSuccessPair>,
    noise: &Noise,
) -> Option<InsertionSuccessPair> {
    match (&left_result, &right_result) {
        (Some(left), Some(right)) => {
            let left_cost = noise.add(left.0.cost + left.1.cost);
            let right_cost = noise.add(right.0.cost + right.1.cost);

            if left_cost < right_cost {
                left_result
            } else {
                right_result
            }
        }
        (Some(_), _) => left_result,
        (None, Some(_)) => right_result,
        _ => None,
    }
}

fn get_new_insertion_ctx(
    insertion_ctx: &InsertionContext,
    seed_job: &Job,
    seed_route_idx: usize,
) -> Result<InsertionContext, String> {
    let mut new_insertion_ctx = insertion_ctx.deep_copy();

    let mut route_ctx = &mut new_insertion_ctx.solution.routes[seed_route_idx];
    let removal_result = route_ctx.route_mut().tour.remove(&seed_job);
    if !removal_result {
        return Err("cannot find job in insertion ctx".to_string());
    }

    // NOTE removed job is not added to the required list as it will be tried
    // to be reinserted later. If insertion fails, the context is discarded

    new_insertion_ctx.problem.constraint.accept_route_state(&mut route_ctx);

    Ok(new_insertion_ctx)
}
