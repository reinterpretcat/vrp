use super::*;
use crate::models::common::{Cost, Schedule};
use crate::models::problem::*;
use crate::models::solution::*;
use crate::models::OP_START_MSG;
use crate::models::{LockOrder, Problem, Solution};
use hashbrown::{HashMap, HashSet};
use rosomaxa::prelude::Environment;
use std::sync::Arc;

type ActivityPlace = crate::models::solution::Place;

/// Creates insertion context from existing solution.
pub fn create_insertion_context(problem: Arc<Problem>, environment: Arc<Environment>) -> InsertionContext {
    let mut locked: HashSet<Job> = Default::default();
    let mut reserved: HashSet<Job> = Default::default();
    let mut ignored: HashSet<Job> = Default::default();
    let mut unassigned: HashMap<Job, UnassignmentInfo> = Default::default();
    let mut routes: Vec<RouteContext> = Default::default();
    let mut registry = Registry::new(&problem.fleet, environment.random.clone());
    let state = Default::default();

    let mut sequence_job_usage: HashMap<Job, usize> = Default::default();

    problem.locks.iter().for_each(|lock| {
        let actor = registry.available().find(|a| (lock.condition_fn)(a.as_ref()));
        match (actor, lock.is_lazy) {
            (Some(actor), false) => {
                registry.use_actor(&actor);
                let mut route_ctx = RouteContext::new(actor);
                let start = route_ctx.route().tour.start().unwrap_or_else(|| panic!("{}", OP_START_MSG)).place.location;

                let create_activity = |single: Arc<Single>, previous_location: usize| {
                    assert_eq!(single.places.len(), 1);

                    let place_idx = 0;
                    let place = &single.places[place_idx];
                    assert_eq!(place.times.len(), 1);

                    let time = single.places.first().unwrap().times.first().unwrap();
                    let time = time
                        .as_time_window()
                        .unwrap_or_else(|| panic!("Job with no time window is not supported in locks"));

                    Activity {
                        place: ActivityPlace {
                            idx: place_idx,
                            location: place.location.unwrap_or(previous_location),
                            duration: place.duration,
                            time,
                        },
                        schedule: Schedule { arrival: 0.0, departure: 0.0 },
                        job: Some(single),
                        commute: None,
                    }
                };

                lock.details.iter().fold(start, |acc, detail| {
                    match detail.order {
                        LockOrder::Any => reserved.extend(detail.jobs.iter().cloned()),
                        _ => locked.extend(detail.jobs.iter().cloned()),
                    }

                    detail.jobs.iter().fold(acc, |acc, job| {
                        let activity = match job {
                            Job::Single(single) => create_activity(single.clone(), acc),
                            Job::Multi(multi) => {
                                let idx = sequence_job_usage.get(job).cloned().unwrap_or(0);
                                sequence_job_usage.insert(job.clone(), idx + 1);
                                create_activity(multi.jobs.get(idx).unwrap().clone(), acc)
                            }
                        };
                        let last_location = activity.place.location;
                        route_ctx.route_mut().tour.insert_last(activity);

                        last_location
                    })
                });

                problem.goal.accept_route_state(&mut route_ctx);

                routes.push(route_ctx);
            }
            (None, false) => {
                unassigned.extend(
                    lock.details.iter().flat_map(|d| d.jobs.iter().cloned().map(|j| (j, UnassignmentInfo::Unknown))),
                );
            }
            (_, _) => {
                locked.extend(lock.details.iter().flat_map(|d| d.jobs.iter().cloned()));
                ignored.extend(lock.details.iter().flat_map(|d| d.jobs.iter().cloned()));
            }
        }
    });

    // NOTE all services from sequence should be used in init route or none of them
    sequence_job_usage.iter().for_each(|(job, usage)| {
        assert_eq!(job.to_multi().jobs.len(), *usage);
    });

    let required = problem
        .jobs
        .all()
        .filter(|job| locked.get(job).is_none() && reserved.get(job).is_none() && unassigned.get(job).is_none())
        .collect();

    let registry = RegistryContext::new(problem.goal.as_ref(), registry);

    let mut insertion_ctx = InsertionContext {
        problem,
        solution: SolutionContext { required, ignored: vec![], unassigned, locked, routes, registry, state },
        environment,
    };

    update_insertion_context(&mut insertion_ctx);

    insertion_ctx
}

/// Creates insertion context from existing solution.
pub fn create_insertion_context_from_solution(
    problem: Arc<Problem>,
    solution: (Solution, Option<Cost>),
    environment: Arc<Environment>,
) -> InsertionContext {
    let required = solution.0.unassigned.iter().map(|(job, _)| job).cloned().collect();
    let locked = problem.locks.iter().fold(HashSet::new(), |mut acc, lock| {
        acc.extend(lock.details.iter().flat_map(|d| d.jobs.iter().cloned()));
        acc
    });

    let mut registry = solution.0.registry.deep_copy();
    let mut routes: Vec<RouteContext> = Vec::new();
    let state = Default::default();

    solution.0.routes.iter().for_each(|route| {
        if route.tour.has_jobs() {
            routes.push(RouteContext::new_with_state(route.deep_copy(), RouteState::default()));
            registry.use_actor(&route.actor);
        } else {
            registry.free_actor(&route.actor);
        }
    });

    let registry = RegistryContext::new(problem.goal.as_ref(), registry);

    let mut insertion_ctx = InsertionContext {
        problem,
        solution: SolutionContext {
            required,
            ignored: vec![],
            unassigned: Default::default(),
            locked,
            routes,
            registry,
            state,
        },
        environment,
    };

    update_insertion_context(&mut insertion_ctx);

    insertion_ctx
}

/// Creates an empty insertion context.
pub fn create_empty_insertion_context(problem: Arc<Problem>, environment: Arc<Environment>) -> InsertionContext {
    let registry = Registry::new(problem.fleet.as_ref(), environment.random.clone());
    InsertionContext {
        problem: problem.clone(),
        solution: SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            locked: Default::default(),
            routes: vec![],
            registry: RegistryContext::new(problem.goal.as_ref(), registry),
            state: Default::default(),
        },
        environment,
    }
}

fn update_insertion_context(insertion_ctx: &mut InsertionContext) {
    // promote required to ignored when necessary
    insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
    // promote all required to unassigned to have a valid statistics
    insertion_ctx
        .solution
        .unassigned
        .extend(insertion_ctx.solution.required.drain(0..).map(|job| (job, UnassignmentInfo::Unknown)));
}
