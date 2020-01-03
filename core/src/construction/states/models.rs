use crate::construction::states::route::RouteState;
use crate::construction::states::{RouteContext, OP_START_MSG};
use crate::models::common::{Cost, Schedule};
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Place, Registry, TourActivity};
use crate::models::{Extras, LockOrder, Problem, Solution};
use crate::utils::Random;
use hashbrown::{HashMap, HashSet};
use std::borrow::Borrow;
use std::ops::Deref;
use std::sync::Arc;

/// Specifies insertion result variant.
pub enum InsertionResult {
    /// Successful insertion result.
    Success(InsertionSuccess),
    /// Insertion failure.
    Failure(InsertionFailure),
}

/// Specifies insertion success result needed to insert job into tour.
pub struct InsertionSuccess {
    /// Specifies delta cost change for the insertion.
    pub cost: Cost,

    /// Original job to be inserted.
    pub job: Arc<Job>,

    /// Specifies activities within index where they have to be inserted.
    pub activities: Vec<(TourActivity, usize)>,

    /// Specifies route context where insertion happens.
    pub context: RouteContext,
}

/// Specifies insertion failure.
pub struct InsertionFailure {
    /// Failed constraint code.
    pub constraint: i32,
}

/// Specifies insertion context for activity.
pub struct ActivityContext<'a> {
    /// Activity insertion index.
    pub index: usize,

    /// Previous activity.
    pub prev: &'a TourActivity,

    /// Target activity.
    pub target: &'a TourActivity,

    /// Next activity. Absent if tour is open and target activity inserted last.
    pub next: Option<&'a TourActivity>,
}

/// Contains information needed to performed insertions in solution.
pub struct InsertionContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Solution context.
    pub solution: SolutionContext,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
}

impl InsertionContext {
    /// Creates insertion context from existing solution.
    pub fn new(problem: Arc<Problem>, random: Arc<dyn Random + Send + Sync>) -> Self {
        let mut locked: HashSet<Arc<Job>> = Default::default();
        let mut reserved: HashSet<Arc<Job>> = Default::default();
        let mut unassigned: HashMap<Arc<Job>, i32> = Default::default();
        let mut routes: Vec<RouteContext> = Default::default();
        let mut registry = Registry::new(&problem.fleet);

        let mut sequence_job_usage: HashMap<Arc<Job>, usize> = Default::default();

        problem.locks.iter().for_each(|lock| {
            let actor = registry.available().find(|a| lock.condition.deref()(a.as_ref()));

            if let Some(actor) = actor {
                registry.use_actor(&actor);
                let mut route_ctx = RouteContext::new(actor);
                let start = route_ctx.route.tour.start().unwrap_or_else(|| panic!(OP_START_MSG)).place.location;

                let create_activity = |single: Arc<Single>, previous_location: usize| {
                    assert_eq!(single.places.len(), 1);
                    assert_eq!(single.places.first().unwrap().times.len(), 1);

                    let place = single.places.first().unwrap();
                    let time = single.places.first().unwrap().times.first().unwrap();

                    Activity {
                        place: Place {
                            location: place.location.unwrap_or(previous_location),
                            duration: place.duration,
                            time: time.clone(),
                        },
                        schedule: Schedule { arrival: 0.0, departure: 0.0 },
                        job: Some(Arc::new(Job::Single(single))),
                    }
                };

                lock.details.iter().fold(start, |acc, detail| {
                    match detail.order {
                        LockOrder::Any => reserved.extend(detail.jobs.iter().cloned()),
                        _ => locked.extend(detail.jobs.iter().cloned()),
                    }

                    detail.jobs.iter().fold(acc, |acc, job| {
                        let activity = match job.as_ref() {
                            Job::Single(single) => create_activity(single.clone(), acc),
                            Job::Multi(multi) => {
                                let idx = sequence_job_usage.get(job).cloned().unwrap_or(0);
                                sequence_job_usage.insert(job.clone(), idx + 1);
                                create_activity(multi.jobs.get(idx).unwrap().clone(), acc)
                            }
                        };
                        let last_location = activity.place.location;
                        route_ctx.route_mut().tour.insert_last(Box::new(activity));

                        last_location
                    })
                });

                problem.constraint.accept_route_state(&mut route_ctx);

                routes.push(route_ctx);
            } else {
                lock.details.iter().for_each(|detail| {
                    detail.jobs.iter().for_each(|job| {
                        // TODO what reason code to use?
                        unassigned.insert(job.clone(), 0);
                    });
                });
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

        let mut ctx = InsertionContext {
            problem: problem.clone(),
            solution: SolutionContext { required, ignored: vec![], unassigned, locked, routes, registry },
            random: random.clone(),
        };

        problem.constraint.accept_solution_state(&mut ctx.solution);

        ctx
    }

    /// Creates insertion context from existing solution.
    pub fn new_from_solution(
        problem: Arc<Problem>,
        solution: (Arc<Solution>, Option<Cost>),
        random: Arc<dyn Random + Send + Sync>,
    ) -> Self {
        let jobs: Vec<Arc<Job>> = solution.0.unassigned.iter().map(|(job, _)| job.clone()).collect();
        let unassigned = Default::default();
        let locked = problem.locks.iter().fold(HashSet::new(), |mut acc, lock| {
            acc.extend(lock.details.iter().flat_map(|d| d.jobs.iter().cloned()));
            acc
        });

        let mut registry = solution.0.registry.deep_copy();
        let mut routes: Vec<RouteContext> = Vec::new();

        solution.0.routes.iter().for_each(|route| {
            if route.tour.has_jobs() {
                let mut route_ctx =
                    RouteContext { route: Arc::new(route.deep_copy()), state: Arc::new(RouteState::default()) };
                problem.constraint.accept_route_state(&mut route_ctx);
                routes.push(route_ctx);
            } else {
                registry.free_actor(&route.actor);
            }
        });

        InsertionContext {
            problem: problem.clone(),
            solution: SolutionContext { required: jobs, ignored: vec![], unassigned, locked, routes, registry },
            random,
        }
    }

    /// Restores valid context state.
    pub fn restore(&mut self) {
        let constraint = self.problem.constraint.clone();
        // NOTE Run first accept solution as it can change existing routes
        // by moving jobs from/to required/ignored jobs.
        // if this happens, accept route state will fix timing/capacity after it
        constraint.accept_solution_state(&mut self.solution);

        self.remove_empty_routes();

        self.solution.routes.iter_mut().for_each(|route_ctx| {
            constraint.accept_route_state(route_ctx);
        });
    }

    pub fn deep_copy(&self) -> Self {
        InsertionContext {
            problem: self.problem.clone(),
            solution: self.solution.deep_copy(),
            random: self.random.clone(),
        }
    }

    /// Removes empty routes from solution context.
    fn remove_empty_routes(&mut self) {
        let registry = &mut self.solution.registry;
        self.solution.routes.retain(|rc| {
            if rc.route.tour.has_jobs() {
                true
            } else {
                registry.free_actor(&rc.route.actor);
                false
            }
        });
    }
}

/// Contains information regarding insertion solution.
pub struct SolutionContext {
    /// List of jobs which require permanent assignment.
    pub required: Vec<Arc<Job>>,

    /// List of jobs which at the moment does not require assignment and might be ignored.
    pub ignored: Vec<Arc<Job>>,

    /// Map of jobs which cannot be assigned and within reason code.
    pub unassigned: HashMap<Arc<Job>, i32>,

    /// Specifies jobs which should not be affected by ruin.
    pub locked: HashSet<Arc<Job>>,

    /// Set of routes within their state.
    pub routes: Vec<RouteContext>,

    /// Keeps track of used resources.
    pub registry: Registry,
}

impl SolutionContext {
    pub fn to_solution(&self, extras: Arc<Extras>) -> Solution {
        Solution {
            registry: self.registry.deep_copy(),
            routes: self.routes.iter().map(|rc| rc.route.deep_copy()).collect(),
            unassigned: self.unassigned.clone(),
            extras,
        }
    }

    pub fn deep_copy(&self) -> Self {
        Self {
            required: self.required.clone(),
            ignored: self.ignored.clone(),
            unassigned: self.unassigned.clone(),
            locked: self.locked.clone(),
            routes: self.routes.iter().map(|rc| rc.deep_copy()).collect(),
            registry: self.registry.deep_copy(),
        }
    }
}

impl InsertionResult {
    pub fn make_success(
        cost: Cost,
        job: Arc<Job>,
        activities: Vec<(TourActivity, usize)>,
        route_ctx: RouteContext,
    ) -> Self {
        Self::Success(InsertionSuccess { cost, job, activities, context: route_ctx })
    }

    /// Creates result which represents insertion failure.
    pub fn make_failure() -> Self {
        Self::make_failure_with_code(0)
    }

    /// Creates result which represents insertion failure with given code.
    pub fn make_failure_with_code(code: i32) -> Self {
        Self::Failure(InsertionFailure { constraint: code })
    }

    /// Compares two insertion results and returns the cheapest by cost.
    pub fn choose_best_result(left: Self, right: Self) -> Self {
        match (left.borrow(), right.borrow()) {
            (Self::Success(_), Self::Failure(_)) => left,
            (Self::Failure(_), Self::Success(_)) => right,
            (Self::Success(lhs), Self::Success(rhs)) => {
                if lhs.cost > rhs.cost {
                    right
                } else {
                    left
                }
            }
            _ => right,
        }
    }
}
