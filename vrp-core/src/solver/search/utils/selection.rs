use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::Problem;
use crate::models::common::Profile;
use crate::models::problem::Job;
use crate::solver::search::TabuList;
use crate::utils::Either;
use rosomaxa::prelude::Random;
use std::collections::HashMap;
use std::iter::{empty, once};

pub(crate) fn get_route_jobs(solution: &SolutionContext) -> HashMap<Job, usize> {
    solution
        .routes
        .iter()
        .enumerate()
        .flat_map(|(route_idx, route_ctx)| route_ctx.route().tour.jobs().cloned().map(move |job| (job, route_idx)))
        .collect()
}

/// Returns seed job within all its neighbours.
pub(crate) fn select_neighbors(problem: &Problem, seed: Option<(Profile, Job)>) -> impl Iterator<Item = Job> + '_ {
    match seed {
        Some((profile, job)) => Either::Left(
            once(job.clone())
                .chain(problem.jobs.neighbors(&profile, &job, Default::default()).map(|(job, _)| job).cloned()),
        ),
        _ => Either::Right(empty()),
    }
}

pub(crate) fn select_seed_job_with_tabu_list(
    insertion_ctx: &InsertionContext,
    tabu_list: &TabuList,
) -> Option<(Profile, usize, Job)> {
    select_seed_job(
        insertion_ctx.solution.routes.as_slice(),
        insertion_ctx.environment.random.as_ref(),
        &|route_ctx| !tabu_list.is_actor_tabu(route_ctx.route().actor.as_ref()),
        &|job| !tabu_list.is_job_tabu(job),
    )
}

/// Selects seed job from existing solution
pub(crate) fn select_seed_job(
    routes: &[RouteContext],
    random: &(dyn Random),
    route_filter: &(dyn Fn(&RouteContext) -> bool),
    job_filter: &(dyn Fn(&Job) -> bool),
) -> Option<(Profile, usize, Job)> {
    if routes.is_empty() {
        return None;
    }

    let initial_route_idx = random.uniform_int(0, (routes.len() - 1) as i32) as usize;
    let mut route_idx = initial_route_idx;

    loop {
        let route_ctx = routes.get(route_idx).unwrap();

        if route_ctx.route().tour.has_jobs() && route_filter(route_ctx) {
            let job = select_random_job(route_ctx, random, job_filter);
            if let Some(job) = job {
                return Some((route_ctx.route().actor.vehicle.profile.clone(), route_idx, job));
            }
        }

        route_idx = (route_idx + 1) % routes.len();
        if route_idx == initial_route_idx {
            break;
        }
    }

    None
}

fn select_random_job(
    route_ctx: &RouteContext,
    random: &(dyn Random),
    job_filter: &(dyn Fn(&Job) -> bool),
) -> Option<Job> {
    let size = route_ctx.route().tour.job_activity_count();
    if size == 0 {
        return None;
    }

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = route_ctx.route().tour.get(ai).and_then(|a| a.retrieve_job());

        if job.as_ref().is_some_and(job_filter) {
            return job;
        }

        ai = (ai + 1) % (size + 1);
        if ai == activity_index {
            break;
        }
    }

    None
}
