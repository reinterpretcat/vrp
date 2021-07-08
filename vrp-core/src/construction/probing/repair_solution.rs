#[cfg(test)]
#[path = "../../../tests/unit/construction/probing/repair_solution_test.rs"]
mod repair_solution_test;

use crate::construction::constraints::ConstraintPipeline;
use crate::construction::heuristics::*;
use crate::models::common::TimeSpan;
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::Activity;
use crate::utils::{compare_floats, unwrap_from_result};
use hashbrown::{HashMap, HashSet};
use std::cmp::Ordering;
use std::sync::Arc;

/// Repairs a feasible solution from another, potentially infeasible.
#[allow(clippy::needless_collect)]
pub fn repair_solution_from_unknown(
    insertion_ctx: &InsertionContext,
    factory: &(dyn Fn() -> InsertionContext),
) -> InsertionContext {
    let mut new_insertion_ctx = factory();
    new_insertion_ctx
        .solution
        .unassigned
        .extend(insertion_ctx.solution.unassigned.iter().map(|(k, v)| (k.clone(), *v)));
    new_insertion_ctx.solution.locked.extend(insertion_ctx.solution.locked.iter().cloned());

    prepare_insertion_ctx(&mut new_insertion_ctx);

    let mut assigned_jobs = get_assigned_jobs(&new_insertion_ctx);
    let constraint = new_insertion_ctx.problem.constraint.clone();

    let unassigned = insertion_ctx
        .solution
        .routes
        .iter()
        .filter(|route_ctx| route_ctx.route.tour.has_jobs())
        .map(|route_ctx| {
            let route_idx = get_new_route_ctx_idx(&mut new_insertion_ctx, route_ctx);

            let synchronized =
                synchronize_jobs(route_ctx, &mut new_insertion_ctx, route_idx, &assigned_jobs, &constraint);

            assigned_jobs.extend(synchronized.keys().cloned());

            new_insertion_ctx.solution.unassigned.drain_filter(|j, _| synchronized.contains_key(j));

            unassign_invalid_multi_jobs(&mut new_insertion_ctx, route_idx, synchronized)
        })
        .flatten()
        .collect::<HashSet<_>>();

    finalize_synchronization(&mut new_insertion_ctx, insertion_ctx, unassigned);

    new_insertion_ctx
}

fn get_new_route_ctx_idx(new_insertion_ctx: &mut InsertionContext, route_ctx: &RouteContext) -> usize {
    if let Some(idx) = new_insertion_ctx
        .solution
        .routes
        .iter()
        .position(|new_route_ctx| new_route_ctx.route.actor == route_ctx.route.actor)
    {
        idx
    } else {
        let mut new_route_ctx =
            new_insertion_ctx.solution.registry.next_with_actor(route_ctx.route.actor.as_ref()).unwrap();

        new_insertion_ctx.solution.registry.use_route(&new_route_ctx);

        // check and set a valid departure shift
        let new_start = new_route_ctx.route_mut().tour.get_mut(0).unwrap();
        let departure = route_ctx.route.tour.start().unwrap().schedule.departure;
        if new_start.place.time.contains(departure) {
            new_start.schedule.departure = departure;
        }
        new_insertion_ctx.problem.constraint.accept_route_state(&mut new_route_ctx);

        new_insertion_ctx.solution.routes.push(new_route_ctx);
        new_insertion_ctx.solution.routes.len() - 1
    }
}

fn get_assigned_jobs(insertion_ctx: &InsertionContext) -> HashSet<Job> {
    insertion_ctx.solution.routes.iter().flat_map(|route_ctx| route_ctx.route.tour.jobs()).collect()
}

fn synchronize_jobs(
    route_ctx: &RouteContext,
    new_insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    assigned_jobs: &HashSet<Job>,
    constraint: &ConstraintPipeline,
) -> HashMap<Job, Vec<Arc<Single>>> {
    let position = InsertionPosition::Last;
    let result_selector = BestResultSelector::default();

    route_ctx
        .route
        .tour
        .all_activities()
        .filter_map(|activity| activity.job.as_ref().map(|job| (job, activity)))
        .filter(|(single, activity)| is_activity_to_single_match(activity, single))
        .filter_map(|(single, activity)| activity.retrieve_job().map(|job| (job, single)))
        .filter(|(job, _)| !assigned_jobs.contains(job))
        .fold(HashMap::default(), |mut synchronized_jobs, (job, single)| {
            let is_already_processed = synchronized_jobs.contains_key(&job) && job.as_single().is_some();

            if !is_already_processed {
                let insertion_result = evaluate_single_constraint_in_route(
                    &job,
                    single,
                    &constraint,
                    new_insertion_ctx,
                    new_insertion_ctx.solution.routes.get(route_idx).unwrap(),
                    position,
                    0.,
                    None,
                    &result_selector,
                );

                if add_single_job(new_insertion_ctx, route_idx, insertion_result, &constraint) {
                    synchronized_jobs.entry(job).or_insert_with(Vec::default).push(single.clone());
                }
            }

            synchronized_jobs
        })
}

fn add_single_job(
    new_insertion_ctx: &mut InsertionContext,
    route_idx: usize,
    insertion_result: InsertionResult,
    constraint: &ConstraintPipeline,
) -> bool {
    if let InsertionResult::Success(success) = insertion_result {
        let mut route_ctx = success.context;
        let route = route_ctx.route_mut();
        assert_eq!(success.activities.len(), 1);
        success.activities.into_iter().for_each(|(activity, index)| {
            route.tour.insert_at(activity, index + 1);
        });

        let total_job_activities = match &success.job {
            Job::Single(_) => 1,
            Job::Multi(multi) => multi.jobs.len(),
        };

        if route.tour.job_activities(&success.job).count() == total_job_activities {
            constraint.accept_insertion(&mut new_insertion_ctx.solution, route_idx, &success.job);
        } else {
            constraint.accept_route_state(new_insertion_ctx.solution.routes.get_mut(route_idx).unwrap());
        }

        true
    } else {
        false
    }
}

fn is_activity_to_single_match(activity: &Activity, single: &Single) -> bool {
    unwrap_from_result(single.places.iter().try_fold(false, |_, place| {
        let is_same_duration = compare_floats(activity.place.duration, place.duration) == Ordering::Equal;
        let is_same_location = place.location.map_or(true, |location| location == activity.place.location);
        let is_same_time_window = place.times.iter().any(|time| {
            match time {
                TimeSpan::Window(tw) => activity.place.time == *tw,
                // TODO support offset time window activities
                TimeSpan::Offset(_) => false,
            }
        });

        if is_same_duration && is_same_location && is_same_time_window {
            Err(true)
        } else {
            Ok(false)
        }
    }))
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
    let assigned = new_insertion_ctx
        .solution
        .routes
        .iter()
        .flat_map(|route_ctx| route_ctx.route.tour.jobs())
        .collect::<HashSet<_>>();

    new_insertion_ctx
        .solution
        .unassigned
        .extend(unassigned.into_iter().chain(insertion_ctx.solution.required.iter().cloned()).map(|job| (job, -1)));

    new_insertion_ctx.solution.required.retain(|j| !assigned.contains(j));

    new_insertion_ctx.restore();

    finalize_insertion_ctx(new_insertion_ctx);
}
