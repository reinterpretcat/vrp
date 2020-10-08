use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::mutation::{select_seed_job, LocalSearch};
use crate::solver::RefinementContext;
use crate::utils::{map_reduce, Random};
use std::ops::Range;
use std::sync::Arc;

/// A local search operator which tries to exchange jobs in best way between different routes.
pub struct ExchangeInterRouteBest {
    _exchange_job_range: Range<usize>,
}

/// A local search operator which tries to exchange random jobs between different routes.
pub struct ExchangeInterRouteRandom {
    _exchange_job_range: Range<usize>,
}

impl ExchangeInterRouteBest {
    /// Creates a new instance of `ExchangeInterRouteBest`.
    pub fn new(exchange_range: Range<usize>) -> Self {
        Self { _exchange_job_range: exchange_range }
    }
}

impl Default for ExchangeInterRouteBest {
    fn default() -> Self {
        Self { _exchange_job_range: 1..2 }
    }
}

impl LocalSearch for ExchangeInterRouteBest {
    fn try_improve(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        unimplemented!()
    }

    fn try_diversify(
        &self,
        refinement_ctx: &RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<InsertionContext> {
        unimplemented!()
    }
}

struct Noise {
    noise_probability: f64,
    noise_range: (f64, f64),
    random: Arc<dyn Random + Send + Sync>,
}

impl Noise {
    pub fn get_with_noise(&self, value: f64) -> f64 {
        unimplemented!()
    }
}

fn find_insertion_pair(
    insertion_ctx: &InsertionContext,
    seed_job: (usize, Job),
    noise: &Noise,
) -> Option<InsertionContext> {
    let (seed_route_idx, seed_job) = seed_job;
    let locked = &insertion_ctx.solution.locked;

    // bad luck: cannot move locked job
    if locked.contains(&seed_job) {
        return None;
    }

    let new_insertion_ctx =
        fork_insertion_ctx_without_job(insertion_ctx, &seed_job, seed_route_idx).expect("cannot fork insertion ctx");

    let seed_route = new_insertion_ctx.solution.routes.get(seed_route_idx).unwrap();

    let insertion_pair = new_insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != seed_route_idx)
        .fold(Option::<(InsertionSuccess, InsertionSuccess)>::None, |acc, (test_route_idx, test_route_ctx)| {
            map_reduce(
                test_route_ctx.route.tour.jobs().filter(|job| !locked.contains(&job)).collect::<Vec<_>>().as_slice(),
                |test_job| {
                    // try to insert test job into seed tour
                    let seed_insertion =
                        analyze_job_insertion_in_route(&new_insertion_ctx, seed_route, test_job, noise);
                    let seed_success = match seed_insertion {
                        InsertionResult::Failure(_) => return None,
                        InsertionResult::Success(success) => success,
                    };

                    // try to insert seed job into test route
                    let mut test_route_ctx = test_route_ctx.deep_copy();
                    test_route_ctx.route_mut().tour.remove(test_job);
                    new_insertion_ctx.problem.constraint.accept_route_state(&mut test_route_ctx);

                    let test_insertion =
                        analyze_job_insertion_in_route(&new_insertion_ctx, &test_route_ctx, &seed_job, noise);
                    let test_success = match test_insertion {
                        InsertionResult::Failure(_) => return None,
                        InsertionResult::Success(success) => success,
                    };

                    Some((seed_success, test_success))
                },
                || None,
                |left, right| match (&left, &right) {
                    (Some(left), Some(right)) => {
                        // TODO
                        unimplemented!()
                    }
                    (Some(_), _) => left,
                    (None, Some(_)) => right,
                    _ => None,
                },
            );

            acc
        });

    unimplemented!()
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

        compare_with_noise(alternative_insertion, new_insertion, noise)
    })
}

fn compare_with_noise(left_result: InsertionResult, right_result: InsertionResult, noise: &Noise) -> InsertionResult {
    match (&left_result, &right_result) {
        (InsertionResult::Success(left), InsertionResult::Success(right)) => {
            let left_cost = noise.get_with_noise(left.cost);
            let right_cost = noise.get_with_noise(right.cost);

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

fn fork_insertion_ctx_without_job(
    insertion_ctx: &InsertionContext,
    seed_job: &Job,
    seed_route_idx: usize,
) -> Result<InsertionContext, String> {
    let mut new_insertion_ctx = insertion_ctx.deep_copy();

    let removal_result = new_insertion_ctx.solution.routes[seed_route_idx].route_mut().tour.remove(&seed_job);
    if removal_result {
        return Err("cannot find job in insertion ctx".to_string());
    }

    new_insertion_ctx.solution.required.push(seed_job.clone());
    new_insertion_ctx.problem.constraint.accept_solution_state(&mut new_insertion_ctx.solution);

    Ok(new_insertion_ctx)
}
