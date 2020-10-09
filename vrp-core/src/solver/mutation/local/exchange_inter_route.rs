#[cfg(test)]
#[path = "../../../../tests/unit/solver/mutation/local/exchange_inter_route_test.rs"]
mod exchange_inter_route_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::{select_seed_job, LocalSearch};
use crate::solver::RefinementContext;
use crate::utils::{map_reduce, Noise};

/// A local search operator which tries to exchange jobs in best way between different routes.
pub struct ExchangeInterRouteBest {
    noise_probability: f64,
    noise_range: (f64, f64),
}

/// A local search operator which tries to exchange random jobs between different routes.
pub struct ExchangeInterRouteRandom {}

impl ExchangeInterRouteBest {
    /// Creates a new instance of `ExchangeInterRouteBest`.
    pub fn new(noise_probability: f64, noise_range: (f64, f64)) -> Self {
        Self { noise_probability, noise_range }
    }
}

impl Default for ExchangeInterRouteBest {
    fn default() -> Self {
        Self::new(0.1, (0.9, 1.1))
    }
}

impl LocalSearch for ExchangeInterRouteBest {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        if let Some(seed_job) = select_seed_job(insertion_ctx.solution.routes.as_slice(), &insertion_ctx.random) {
            let noise = Noise::new(self.noise_probability, self.noise_range, insertion_ctx.random.clone());
            find_best_insertion_pair(insertion_ctx, seed_job, &noise, {
                let noise = noise.clone();
                Box::new(move |new_insertion_ctx, (seed_route, seed_job), (test_route, test_job)| {
                    // try to insert test job into seed tour
                    let seed_insertion =
                        analyze_job_insertion_in_route(&new_insertion_ctx, seed_route, test_job, &noise);
                    let seed_success = match seed_insertion {
                        InsertionResult::Failure(_) => return None,
                        InsertionResult::Success(success) => success,
                    };

                    // try to insert seed job into test route
                    let mut test_route = test_route.deep_copy();
                    test_route.route_mut().tour.remove(test_job);
                    new_insertion_ctx.problem.constraint.accept_route_state(&mut test_route);

                    let test_insertion =
                        analyze_job_insertion_in_route(&new_insertion_ctx, &test_route, &seed_job, &noise);
                    let test_success = match test_insertion {
                        InsertionResult::Failure(_) => return None,
                        InsertionResult::Success(success) => success,
                    };

                    Some((seed_success, test_success))
                })
            })
        } else {
            None
        }
    }
}

type InsertionSuccessPair = (InsertionSuccess, InsertionSuccess);
type InsertionTargetPair<'a> = (&'a RouteContext, &'a Job);

fn find_best_insertion_pair(
    insertion_ctx: &InsertionContext,
    seed_job: (usize, Job),
    noise: &Noise,
    get_insertion_pair: Box<
        dyn Fn(&InsertionContext, InsertionTargetPair, InsertionTargetPair) -> Option<InsertionSuccessPair>
            + Send
            + Sync,
    >,
) -> Option<InsertionContext> {
    let (seed_route_idx, seed_job) = seed_job;
    let locked = &insertion_ctx.solution.locked;

    // bad luck: cannot move locked job
    if locked.contains(&seed_job) {
        return None;
    }

    let new_insertion_ctx = get_new_insertion_ctx(insertion_ctx, &seed_job, seed_route_idx).unwrap();
    let seed_route = new_insertion_ctx.solution.routes.get(seed_route_idx).unwrap();

    let insertion_pair = new_insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != seed_route_idx)
        .fold(Option::<InsertionSuccessPair>::None, |acc, (_, test_route_ctx)| {
            let new_result = map_reduce(
                test_route_ctx.route.tour.jobs().filter(|job| !locked.contains(&job)).collect::<Vec<_>>().as_slice(),
                |test_job| get_insertion_pair(&new_insertion_ctx, (seed_route, &seed_job), (&test_route_ctx, test_job)),
                || None,
                |left, right| reduce_pair_with_noise(left, right, noise),
            );

            reduce_pair_with_noise(acc, new_result, noise)
        });

    if let Some(insertion_pair) = insertion_pair {
        let mut new_insertion_ctx = new_insertion_ctx;
        apply_insertion_success(&mut new_insertion_ctx, insertion_pair.0);
        apply_insertion_success(&mut new_insertion_ctx, insertion_pair.1);
        finalize_insertion_ctx(&mut new_insertion_ctx);

        Some(new_insertion_ctx)
    } else {
        None
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
        RouteContext { route: insertion_success.context.route.clone(), state: insertion_success.context.state.clone() };

    apply_insertion_result(insertion_ctx, InsertionResult::Success(insertion_success))
}

fn analyze_job_insertion_in_route(
    new_insertion_ctx: &InsertionContext,
    route_ctx: &RouteContext,
    job: &Job,
    noise: &Noise,
) -> InsertionResult {
    let route_leg_count = route_ctx.route.tour.legs().count();

    (0..route_leg_count).fold(InsertionResult::make_failure(), |alternative_insertion, idx| {
        let new_insertion = evaluate_job_insertion_in_route(
            job,
            &new_insertion_ctx,
            &route_ctx,
            InsertionPosition::Concrete(idx),
            InsertionResult::make_failure(),
        );

        compare_insertion_result_with_noise(alternative_insertion, new_insertion, noise)
    })
}

fn compare_insertion_result_with_noise(
    left_result: InsertionResult,
    right_result: InsertionResult,
    noise: &Noise,
) -> InsertionResult {
    match (&left_result, &right_result) {
        (InsertionResult::Success(left), InsertionResult::Success(right)) => {
            let left_cost = noise.add(left.cost);
            let right_cost = noise.add(right.cost);

            if left_cost < right_cost {
                left_result
            } else {
                right_result
            }
        }
        (_, InsertionResult::Success(_)) => right_result,
        (_, InsertionResult::Failure(_)) => left_result,
    }
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
