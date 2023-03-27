#[cfg(test)]
#[path = "../../../../tests/unit/solver/search/local/exchange_sequence_test.rs"]
mod exchange_sequence_test;

use crate::construction::heuristics::*;
use crate::models::problem::Job;
use crate::solver::search::LocalOperator;
use crate::solver::RefinementContext;
use hashbrown::HashSet;
use rand::prelude::SliceRandom;
use rosomaxa::prelude::*;

const MIN_JOBS: usize = 2;

/// A local search operator which tries to exchange sequence of jobs between routes.
pub struct ExchangeSequence {
    max_sequence_size: usize,
    reverse_prob: f64,
    shuffle_prob: f64,
}

impl ExchangeSequence {
    /// Creates a new instance of `ExchangeSequence`.
    pub fn new(max_sequence_size: usize, reverse_prob: f64, shuffle_prob: f64) -> Self {
        assert!(max_sequence_size >= MIN_JOBS);

        Self { max_sequence_size, reverse_prob, shuffle_prob }
    }
}

impl Default for ExchangeSequence {
    fn default() -> Self {
        Self::new(6, 0.5, 0.01)
    }
}

impl LocalOperator for ExchangeSequence {
    fn explore(&self, _: &RefinementContext, insertion_ctx: &InsertionContext) -> Option<InsertionContext> {
        let route_indices = get_route_indices(insertion_ctx);

        if route_indices.is_empty() {
            return None;
        }

        let mut insertion_ctx = insertion_ctx.deep_copy();

        exchange_jobs(
            &mut insertion_ctx,
            route_indices.as_slice(),
            self.max_sequence_size,
            self.reverse_prob,
            self.shuffle_prob,
        );

        Some(insertion_ctx)
    }
}

fn get_route_indices(insertion_ctx: &InsertionContext) -> Vec<usize> {
    insertion_ctx
        .solution
        .routes
        .iter()
        .enumerate()
        .filter_map(|(idx, route_ctx)| {
            let locked_jobs =
                route_ctx.route().tour.jobs().filter(|job| insertion_ctx.solution.locked.contains(job)).count();
            let has_enough_jobs = (route_ctx.route().tour.job_count() - locked_jobs) >= MIN_JOBS;

            if has_enough_jobs {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn exchange_jobs(
    insertion_ctx: &mut InsertionContext,
    route_indices: &[usize],
    max_sequence_size: usize,
    reverse_prob: f64,
    shuffle_prob: f64,
) {
    let get_route_idx = |insertion_ctx: &InsertionContext| {
        let idx = insertion_ctx.environment.random.uniform_int(0, route_indices.len() as i32 - 1) as usize;
        route_indices.get(idx).cloned().unwrap()
    };

    let get_sequence_size = |insertion_ctx: &InsertionContext, route_idx: usize| {
        let job_count = get_route_ctx(insertion_ctx, route_idx).route().tour.job_count().min(max_sequence_size);
        insertion_ctx.environment.random.uniform_int(MIN_JOBS as i32, job_count as i32) as usize
    };

    let first_route_idx = get_route_idx(insertion_ctx);
    let first_sequence_size = get_sequence_size(insertion_ctx, first_route_idx);
    let first_jobs = extract_jobs(insertion_ctx, first_route_idx, first_sequence_size);

    let second_route_idx = get_route_idx(insertion_ctx);

    if first_route_idx != second_route_idx {
        let second_sequence_size = get_sequence_size(insertion_ctx, second_route_idx);
        let second_jobs = extract_jobs(insertion_ctx, second_route_idx, second_sequence_size);

        insert_jobs(insertion_ctx, first_route_idx, second_jobs, reverse_prob, shuffle_prob);
        insert_jobs(insertion_ctx, second_route_idx, first_jobs, reverse_prob, shuffle_prob);
    } else {
        insert_jobs(insertion_ctx, first_route_idx, first_jobs, reverse_prob, shuffle_prob);
    }

    finalize_insertion_ctx(insertion_ctx);
}

fn extract_jobs(insertion_ctx: &mut InsertionContext, route_idx: usize, sequence_size: usize) -> Vec<Job> {
    let locked = &insertion_ctx.solution.locked;
    let route_ctx = insertion_ctx.solution.routes.get_mut(route_idx).unwrap();
    let job_count = route_ctx.route().tour.job_count();

    assert!(job_count >= sequence_size);

    // get jobs in the exact order as they appear first time in the tour
    let (_, jobs) = route_ctx.route().tour.all_activities().filter_map(|activity| activity.retrieve_job()).fold(
        (HashSet::<Job>::default(), Vec::with_capacity(job_count)),
        |(mut set, mut vec), job| {
            if !set.contains(&job) && !locked.contains(&job) {
                vec.push(job.clone());
                set.insert(job);
            }

            (set, vec)
        },
    );

    let sequence_size = sequence_size.min(jobs.len());
    let last_index = jobs.len() - sequence_size;
    let start_index = insertion_ctx.environment.random.uniform_int(0, last_index as i32) as usize;

    let removed =
        (start_index..(start_index + sequence_size)).fold(Vec::with_capacity(sequence_size), |mut acc, index| {
            let job = jobs.get(index).unwrap();
            assert!(route_ctx.route_mut().tour.remove(job));
            acc.push(job.clone());

            acc
        });

    insertion_ctx.problem.goal.accept_route_state(route_ctx);

    removed
}

fn insert_jobs(
    insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    jobs: Vec<Job>,
    reverse_prob: f64,
    shuffle_prob: f64,
) {
    let random = &insertion_ctx.environment.random;
    let leg_selection = LegSelection::Stochastic(random.clone());
    let result_selector = BestResultSelector::default();

    let mut jobs = jobs;
    match (random.is_hit(reverse_prob), random.is_hit(shuffle_prob)) {
        (true, _) => {
            jobs.reverse();
        }
        (_, true) => {
            jobs.shuffle(&mut random.get_rng());
        }
        _ => {}
    };

    let start_index = random
        .uniform_int(0, get_route_ctx(insertion_ctx, route_idx).route().tour.job_activity_count() as i32)
        as usize;

    let (failures, _) = jobs.into_iter().fold((Vec::new(), start_index), |(mut unassigned, start_index), job| {
        let eval_ctx = EvaluationContext {
            goal: &insertion_ctx.problem.goal,
            job: &job,
            leg_selection: &leg_selection,
            result_selector: &result_selector,
        };

        // reevaluate last insertion point
        let last_index = get_route_ctx(insertion_ctx, route_idx).route().tour.job_activity_count();
        // try to find success insertion starting from given point
        let (result, start_index) = unwrap_from_result((start_index..=last_index).try_fold(
            (InsertionResult::make_failure(), start_index),
            |_, insertion_idx| {
                let insertion = eval_job_insertion_in_route(
                    insertion_ctx,
                    &eval_ctx,
                    get_route_ctx(insertion_ctx, route_idx),
                    InsertionPosition::Concrete(insertion_idx),
                    // NOTE we don't try to insert the best, so alternative is a failure
                    InsertionResult::make_failure(),
                );

                match &insertion {
                    InsertionResult::Failure(_) => Ok((insertion, insertion_idx)),
                    InsertionResult::Success(_) => Err((insertion, insertion_idx)),
                }
            },
        ));

        match result {
            InsertionResult::Success(success) => {
                apply_insertion_success(insertion_ctx, success);
            }
            InsertionResult::Failure(failure) => unassigned.push((job, failure)),
        }

        (unassigned, start_index + 1)
    });

    insertion_ctx.solution.unassigned.extend(failures.into_iter().map(|(job, failure)| {
        let code = UnassignmentInfo::Simple(failure.constraint);
        let job = failure.job.unwrap_or(job);
        (job, code)
    }));
}

fn get_route_ctx(insertion_ctx: &InsertionContext, route_idx: usize) -> &RouteContext {
    insertion_ctx.solution.routes.get(route_idx).unwrap()
}
