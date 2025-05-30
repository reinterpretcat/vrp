#[cfg(test)]
#[path = "../../../tests/unit/construction/probing/repair_solution_test.rs"]
mod repair_solution_test;

use crate::construction::heuristics::*;
use crate::models::GoalContext;
use crate::models::common::TimeSpan;
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::Activity;
use rosomaxa::prelude::*;
use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::sync::Arc;

/// Repairs a feasible solution from another, potentially infeasible.
#[allow(clippy::needless_collect)]
pub fn repair_solution_from_unknown(
    insertion_ctx: &InsertionContext,
    factory: &(dyn Fn() -> InsertionContext),
) -> InsertionContext {
    let mut new_insertion_ctx = factory();

    prepare_insertion_ctx(&mut new_insertion_ctx);

    let mut assigned_jobs = get_assigned_jobs(&new_insertion_ctx);
    let goal = new_insertion_ctx.problem.goal.clone();

    let unassigned = insertion_ctx
        .solution
        .routes
        .iter()
        .filter(|route_ctx| route_ctx.route().tour.has_jobs())
        .flat_map(|route_ctx| {
            let route_idx = get_new_route_ctx_idx(&mut new_insertion_ctx, route_ctx);

            let synchronized = synchronize_jobs(route_ctx, &mut new_insertion_ctx, route_idx, &assigned_jobs, &goal);

            assigned_jobs.extend(synchronized.keys().cloned());

            new_insertion_ctx.solution.unassigned.retain(|j, _| !synchronized.contains_key(j));
            new_insertion_ctx.solution.ignored.retain(|j| !synchronized.contains_key(j));
            new_insertion_ctx.solution.required.retain(|j| !synchronized.contains_key(j));

            unassign_invalid_multi_jobs(&mut new_insertion_ctx, route_idx, synchronized)
        })
        .collect::<HashSet<_>>();

    finalize_synchronization(&mut new_insertion_ctx, insertion_ctx, unassigned);

    new_insertion_ctx
}

fn get_new_route_ctx_idx(new_insertion_ctx: &mut InsertionContext, route_ctx: &RouteContext) -> usize {
    match new_insertion_ctx
        .solution
        .routes
        .iter()
        .position(|new_route_ctx| new_route_ctx.route().actor == route_ctx.route().actor)
    {
        Some(idx) => idx,
        _ => {
            let mut new_route_ctx = new_insertion_ctx
                .solution
                .registry
                .get_route(&route_ctx.route().actor)
                .expect("actor is already in use");

            // check and set a valid departure shift
            let new_start = new_route_ctx.route_mut().tour.get_mut(0).unwrap();
            let departure = route_ctx.route().tour.start().unwrap().schedule.departure;
            if new_start.place.time.contains(departure) {
                new_start.schedule.departure = departure;
            }
            new_insertion_ctx.problem.goal.accept_route_state(&mut new_route_ctx);

            new_insertion_ctx.solution.routes.push(new_route_ctx);
            new_insertion_ctx.solution.routes.len() - 1
        }
    }
}

fn get_assigned_jobs(insertion_ctx: &InsertionContext) -> HashSet<Job> {
    insertion_ctx.solution.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs().cloned()).collect()
}

fn synchronize_jobs(
    route_ctx: &RouteContext,
    new_insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    assigned_jobs: &HashSet<Job>,
    goal: &GoalContext,
) -> HashMap<Job, Vec<Arc<Single>>> {
    let position = InsertionPosition::Last;
    let leg_selection = LegSelection::Exhaustive;
    let result_selector = BestResultSelector::default();

    let (synchronized_jobs, _) = route_ctx
        .route()
        .tour
        .all_activities()
        .filter_map(|activity| activity.job.as_ref().map(|job| (job, activity)))
        .filter(|(single, activity)| is_activity_to_single_match(activity, single))
        .filter_map(|(single, activity)| activity.retrieve_job().map(|job| (job, single)))
        .filter(|(job, _)| !assigned_jobs.contains(job))
        .fold(
            (HashMap::default(), HashSet::<Job>::default()),
            |(mut synchronized_jobs, mut invalid_multi_job_ids), (job, single)| {
                let is_already_processed = synchronized_jobs.contains_key(&job) && job.as_single().is_some();
                let is_invalid_multi_job = invalid_multi_job_ids.contains(&job);

                // Skip already processed singles and invalid multi jobs
                if is_already_processed || is_invalid_multi_job {
                    return (synchronized_jobs, invalid_multi_job_ids);
                }

                let eval_ctx = EvaluationContext {
                    goal,
                    job: &job,
                    leg_selection: &leg_selection,
                    result_selector: &result_selector,
                };
                let route_ctx = &new_insertion_ctx.solution.routes[route_idx];

                let insertion_result = eval_single_constraint_in_route(
                    new_insertion_ctx,
                    &eval_ctx,
                    route_ctx,
                    single,
                    position,
                    Default::default(),
                    None,
                );

                match insertion_result {
                    InsertionResult::Success(success) => {
                        apply_insertion_success(new_insertion_ctx, success);
                        synchronized_jobs.entry(job).or_insert_with(Vec::default).push(single.clone());
                    }
                    InsertionResult::Failure(_) if job.as_multi().is_some() => {
                        invalid_multi_job_ids.insert(job.clone());
                    }
                    InsertionResult::Failure(_) => {}
                }

                (synchronized_jobs, invalid_multi_job_ids)
            },
        );

    synchronized_jobs
}

fn is_activity_to_single_match(activity: &Activity, single: &Single) -> bool {
    single
        .places
        .iter()
        .try_fold(false, |_, place| {
            let is_same_duration = activity.place.duration == place.duration;
            let is_same_location = place.location.is_none_or(|location| location == activity.place.location);
            let is_same_time_window = place.times.iter().any(|time| {
                match time {
                    TimeSpan::Window(tw) => activity.place.time == *tw,
                    // TODO support offset time window activities
                    TimeSpan::Offset(_) => false,
                }
            });

            if is_same_duration && is_same_location && is_same_time_window {
                ControlFlow::Break(true)
            } else {
                ControlFlow::Continue(false)
            }
        })
        .unwrap_value()
}

fn unassign_invalid_multi_jobs(
    new_insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    synchronized: HashMap<Job, Vec<Arc<Single>>>,
) -> Vec<Job> {
    let new_route_ctx = new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap();

    synchronized
        .iter()
        .filter_map(|(job, singles)| match job {
            Job::Multi(multi) => Some((job, multi, singles)),
            Job::Single(_) => None,
        })
        .fold(Vec::default(), |mut unassigned, (job, multi, singles)| {
            if multi.jobs.len() != singles.len() || !compare_singles(multi, singles.as_slice()) {
                new_route_ctx.route_mut().tour.remove(job);
                unassigned.push(job.clone());
            }

            unassigned
        })
}

fn compare_singles(multi: &Multi, singles: &[Arc<Single>]) -> bool {
    let job_map = multi
        .jobs
        .iter()
        .enumerate()
        .map(|(idx, single)| (Job::Single(single.clone()), idx))
        .collect::<HashMap<_, _>>();

    let permutation =
        singles.iter().filter_map(|single| job_map.get(&Job::Single(single.clone())).cloned()).collect::<Vec<_>>();

    multi.validate(permutation.as_slice())
}

fn finalize_synchronization(
    new_insertion_ctx: &mut InsertionContext,
    insertion_ctx: &InsertionContext,
    unassigned: HashSet<Job>,
) {
    new_insertion_ctx.solution.unassigned.extend(
        unassigned
            .into_iter()
            .chain(insertion_ctx.solution.required.iter().cloned())
            .map(|job| (job, UnassignmentInfo::Unknown)),
    );

    new_insertion_ctx.restore();

    finalize_insertion_ctx(new_insertion_ctx);
}
