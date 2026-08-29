#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_inter_route_test.rs"]
mod exchange_inter_route_test;

use super::*;
use crate::models::GoalContext;
use crate::models::problem::Job;
use crate::models::solution::Activity;
use crate::solver::RefinementContext;
use crate::solver::search::{LocalOperator, TabuList, select_seed_job};
use crate::utils::Noise;
use rosomaxa::utils::map_reduce;

/// A local search operator which tries to exchange jobs in best way between different routes.
pub struct ExchangeInterRouteBest {
    noise_probability: Float,
    noise_range: (Float, Float),
}

/// A local search operator which tries to exchange random jobs between different routes.
pub struct ExchangeInterRouteRandom {
    noise_probability: Float,
    noise_range: (Float, Float),
}

impl ExchangeInterRouteBest {
    /// Creates a new instance of `ExchangeInterRouteBest`.
    pub fn new(noise_probability: Float, min: Float, max: Float) -> Self {
        Self { noise_probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeInterRouteBest {
    fn default() -> Self {
        Self::new(0.05, -0.25, 0.25)
    }
}

impl LocalOperator for ExchangeInterRouteBest {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        find_best_insertion_pair(
            insertion_ctx,
            Noise::new_with_addition(
                self.noise_probability,
                self.noise_range,
                insertion_ctx.environment.random.clone(),
            ),
            Box::new(|_| true),
            Box::new(|_| true),
        )
    }
}

impl ExchangeInterRouteRandom {
    /// Creates a new instance of `ExchangeInterRouteRandom`.
    pub fn new(noise_probability: Float, min: Float, max: Float) -> Self {
        Self { noise_probability, noise_range: (min, max) }
    }
}

impl Default for ExchangeInterRouteRandom {
    fn default() -> Self {
        Self::new(0.1, -0.25, 0.25)
    }
}

impl LocalOperator for ExchangeInterRouteRandom {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let random = &insertion_ctx.environment.random;
        find_best_insertion_pair(
            insertion_ctx,
            Noise::new_with_addition(self.noise_probability, self.noise_range, random.clone()),
            {
                let random = random.clone();
                Box::new(move |_idx| random.is_head_not_tails())
            },
            {
                let random = random.clone();
                Box::new(move |_idx| random.is_head_not_tails())
            },
        )
    }
}

type InsertionSuccessPair = ((InsertionSuccess, Option<RouteContext>), (InsertionSuccess, Option<RouteContext>));

struct ExchangeCandidate {
    seed_success: InsertionSuccess,
    test_success: InsertionSuccess,
    test_route_idx: usize,
    // Small route sets keep job-level parallelism and retain the evaluated route. Route-level
    // reduction leaves it empty and rebuilds only the winner from the immutable original.
    test_route: Option<RouteContext>,
}

/// Reuses route storage while evaluating different single-job removals from the same original route.
/// Feature state is still fully refreshed after every removal.
struct RouteRemovalCursor<'a> {
    original: &'a RouteContext,
    route: RouteContext,
    removed_activities: Vec<(Activity, usize)>,
}

impl<'a> RouteRemovalCursor<'a> {
    fn new(original: &'a RouteContext) -> Self {
        Self { original, route: original.deep_copy(), removed_activities: Vec::with_capacity(2) }
    }

    fn remove(&mut self, goal: &GoalContext, job: &Job) -> Option<&RouteContext> {
        self.removed_activities.drain(..).for_each(|(activity, index)| {
            self.route.route_mut().tour.insert_at(activity, index);
        });
        self.removed_activities.extend(
            self.original
                .route()
                .tour
                .all_activities()
                .enumerate()
                .filter(|(_, activity)| activity.has_same_job(job))
                .map(|(index, activity)| (activity.deep_copy(), index)),
        );

        if self.removed_activities.is_empty() || !self.route.route_mut().tour.remove(job) {
            self.removed_activities.clear();
            return None;
        }

        goal.accept_route_state(&mut self.route);

        Some(&self.route)
    }
}

// Route-level tasks can reuse removal storage when the search already has enough independent work.
// With fewer routes, job-level tasks provide better parallelism.
const ROUTE_REMOVAL_REUSE_THRESHOLD: usize = 16;

fn find_best_insertion_pair(
    insertion_ctx: &InsertionContext,
    noise: Noise,
    filter_route_indices: Box<dyn Fn(usize) -> bool + Send + Sync>,
    filter_jobs_indices: Box<dyn Fn(usize) -> bool + Send + Sync>,
) -> Option<InsertionContext> {
    let mut tabu_list = TabuList::from(insertion_ctx);
    let locked = &insertion_ctx.solution.locked;
    let seed = select_seed_job(
        insertion_ctx.solution.routes.as_slice(),
        insertion_ctx.environment.random.as_ref(),
        &|route_ctx| !tabu_list.is_actor_tabu(route_ctx.route().actor.as_ref()),
        &|job| !tabu_list.is_job_tabu(job) && !locked.contains(job),
    );

    if let Some((_, seed_route_idx, seed_job)) = seed {
        let new_insertion_ctx = get_new_insertion_ctx(insertion_ctx, &seed_job, seed_route_idx).unwrap();
        let seed_route = new_insertion_ctx.solution.routes.get(seed_route_idx).unwrap();
        let leg_selection = LegSelection::Stochastic(insertion_ctx.environment.random.clone());
        let result_selector = NoiseResultSelector::new(noise.clone());

        let routes = new_insertion_ctx
            .solution
            .routes
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != seed_route_idx && filter_route_indices(*idx));
        let evaluate = |test_route_idx: usize, test_route: &RouteContext, test_job: &Job| {
            evaluate_exchange_candidate(
                &new_insertion_ctx,
                seed_route,
                &seed_job,
                test_route_idx,
                test_route,
                test_job,
                &leg_selection,
                &result_selector,
            )
        };
        let insertion_pair = if new_insertion_ctx.solution.routes.len() <= ROUTE_REMOVAL_REUSE_THRESHOLD {
            routes.fold(Option::<ExchangeCandidate>::None, |acc, (test_route_idx, test_route)| {
                let test_jobs = test_route
                    .route()
                    .tour
                    .jobs()
                    .enumerate()
                    .filter(|(idx, job)| !locked.contains(*job) && filter_jobs_indices(*idx))
                    .collect::<Vec<_>>();
                let result = map_reduce(
                    test_jobs.as_slice(),
                    |(_, test_job)| evaluate(test_route_idx, test_route, test_job),
                    || None,
                    |left, right| reduce_candidate_with_noise(left, right, &noise),
                );

                reduce_candidate_with_noise(acc, result, &noise)
            })
        } else {
            let routes = routes.collect::<Vec<_>>();

            map_reduce(
                routes.as_slice(),
                |(test_route_idx, test_route)| {
                    let mut removal = RouteRemovalCursor::new(test_route);

                    test_route
                        .route()
                        .tour
                        .jobs()
                        .enumerate()
                        .filter(|(idx, job)| !locked.contains(*job) && filter_jobs_indices(*idx))
                        .fold(None, |best, (_, test_job)| {
                            let candidate = evaluate_exchange_candidate_reusing_route(
                                &new_insertion_ctx,
                                seed_route,
                                &seed_job,
                                *test_route_idx,
                                test_job,
                                &mut removal,
                                &leg_selection,
                                &result_selector,
                            );

                            reduce_candidate_with_noise(best, candidate, &noise)
                        })
                },
                || None,
                |left, right| reduce_candidate_with_noise(left, right, &noise),
            )
        };

        if let Some(insertion_pair) =
            insertion_pair.and_then(|candidate| materialize_insertion_pair(&new_insertion_ctx, candidate))
        {
            let mut new_insertion_ctx = new_insertion_ctx;

            for (success, _) in [&insertion_pair.0, &insertion_pair.1] {
                tabu_list.add_job(success.job.clone())
            }

            apply_insertion_with_route(&mut new_insertion_ctx, insertion_pair.0);
            apply_insertion_with_route(&mut new_insertion_ctx, insertion_pair.1);
            finalize_insertion_ctx(&mut new_insertion_ctx);

            tabu_list.inject(&mut new_insertion_ctx);

            return Some(new_insertion_ctx);
        }
    }

    None
}

fn evaluate_exchange_candidate(
    insertion_ctx: &InsertionContext,
    seed_route: &RouteContext,
    seed_job: &Job,
    test_route_idx: usize,
    test_route: &RouteContext,
    test_job: &Job,
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
) -> Option<ExchangeCandidate> {
    // try to insert test job into seed tour
    let seed_success = test_job_insertion(insertion_ctx, seed_route, test_job, leg_selection, result_selector)?;

    // try to insert seed job into test route
    let mut test_route = test_route.deep_copy();
    // NOTE would be nice to add job to list of required
    test_route.route_mut().tour.remove(test_job);
    insertion_ctx.problem.goal.accept_route_state(&mut test_route);

    let test_success = test_job_insertion(insertion_ctx, &test_route, seed_job, leg_selection, result_selector)?;

    Some(ExchangeCandidate { seed_success, test_success, test_route_idx, test_route: Some(test_route) })
}

fn evaluate_exchange_candidate_reusing_route(
    insertion_ctx: &InsertionContext,
    seed_route: &RouteContext,
    seed_job: &Job,
    test_route_idx: usize,
    test_job: &Job,
    removal: &mut RouteRemovalCursor,
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
) -> Option<ExchangeCandidate> {
    let seed_success = test_job_insertion(insertion_ctx, seed_route, test_job, leg_selection, result_selector)?;
    let test_route = removal.remove(&insertion_ctx.problem.goal, test_job)?;
    let test_success = test_job_insertion(insertion_ctx, test_route, seed_job, leg_selection, result_selector)?;

    Some(ExchangeCandidate { seed_success, test_success, test_route_idx, test_route: None })
}

fn materialize_insertion_pair(
    insertion_ctx: &InsertionContext,
    candidate: ExchangeCandidate,
) -> Option<InsertionSuccessPair> {
    let ExchangeCandidate { seed_success, test_success, test_route_idx, test_route } = candidate;
    let test_route = match test_route {
        Some(test_route) => test_route,
        None => {
            let mut test_route = insertion_ctx.solution.routes.get(test_route_idx)?.deep_copy();
            if !test_route.route_mut().tour.remove(&seed_success.job) {
                return None;
            }
            insertion_ctx.problem.goal.accept_route_state(&mut test_route);
            test_route
        }
    };

    Some(((seed_success, None), (test_success, Some(test_route))))
}

fn test_job_insertion(
    insertion_ctx: &InsertionContext,
    route_ctx: &RouteContext,
    job: &Job,
    leg_selection: &LegSelection,
    result_selector: &dyn ResultSelector,
) -> Option<InsertionSuccess> {
    let eval_ctx = EvaluationContext { goal: &insertion_ctx.problem.goal, job, leg_selection, result_selector };

    let insertion = eval_job_insertion_in_route(
        insertion_ctx,
        &eval_ctx,
        route_ctx,
        InsertionPosition::Any,
        InsertionResult::make_failure(),
    );

    match insertion {
        InsertionResult::Failure(_) => None,
        InsertionResult::Success(success) => Some(success),
    }
}

fn get_insertion_cost_with_noise(candidate: &ExchangeCandidate, noise: &Noise) -> InsertionCost {
    noise.generate_multi((&candidate.seed_success.cost + &candidate.test_success.cost).iter()).collect()
}

fn reduce_candidate_with_noise(
    left_result: Option<ExchangeCandidate>,
    right_result: Option<ExchangeCandidate>,
    noise: &Noise,
) -> Option<ExchangeCandidate> {
    match (&left_result, &right_result) {
        (Some(left), Some(right)) => {
            let left_cost = get_insertion_cost_with_noise(left, noise);
            let right_cost = get_insertion_cost_with_noise(right, noise);

            if left_cost < right_cost { left_result } else { right_result }
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
) -> Result<InsertionContext, GenericError> {
    let mut new_insertion_ctx = insertion_ctx.deep_copy();

    let route_ctx = &mut new_insertion_ctx.solution.routes[seed_route_idx];
    let removal_result = route_ctx.route_mut().tour.remove(seed_job);
    if !removal_result {
        return Err("cannot find job in insertion ctx".into());
    }

    // NOTE removed job is not added to the required list as it will be tried
    // to be reinserted later. If insertion fails, the context is discarded

    new_insertion_ctx.problem.goal.accept_route_state(route_ctx);

    Ok(new_insertion_ctx)
}
