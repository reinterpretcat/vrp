use crate::construction::states::route::RouteState;
use crate::models::common::{Cost, Schedule};
use crate::models::problem::{Actor, Job};
use crate::models::solution::{Activity, Place, Registry, Route, Tour, TourActivity};
use crate::models::{Extras, Problem, Solution};
use crate::utils::Random;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::{Arc, RwLock};

/// Specifies insertion result.
pub enum InsertionResult {
    Success(InsertionSuccess),
    Failure(InsertionFailure),
}

/// Specifies insertion result needed to insert job into tour.
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

/// Provides the way to get some meta information about insertion progress.
#[derive(Clone)]
pub struct InsertionProgress {
    /// Specifies best known cost depending on context.
    pub cost: Option<Cost>,

    /// Specifies solution completeness.
    pub completeness: f64,

    /// Total amount of jobs.
    pub total: usize,
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

/// Specifies insertion context for route.
#[derive(Clone)]
pub struct RouteContext {
    /// Used route.
    pub route: Arc<RwLock<Route>>,

    /// Insertion state.
    pub state: Arc<RwLock<RouteState>>,
}

/// Contains information needed to performed insertions in solution.
pub struct InsertionContext {
    /// Solution progress.
    pub progress: InsertionProgress,

    /// Original problem.
    pub problem: Arc<Problem>,

    /// Solution context.
    pub solution: SolutionContext,

    /// Specifies jobs which should not be affected.
    pub locked: Arc<HashSet<Arc<Job>>>,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
}

impl InsertionContext {
    /// Creates insertion context from existing solution.
    pub fn new(problem: Arc<Problem>, random: Arc<dyn Random + Send + Sync>) -> Self {
        InsertionContext {
            progress: InsertionProgress { cost: None, completeness: 0., total: problem.jobs.size() },
            problem: problem.clone(),
            solution: SolutionContext {
                required: problem.jobs.all().collect(),
                ignored: vec![],
                unassigned: Default::default(),
                routes: Default::default(),
                registry: Registry::new(&problem.fleet),
            },
            locked: Self::get_locked_jobs(&problem),
            random: random.clone(),
        }
    }

    /// Creates insertion context from existing solution.
    pub fn new_from_solution(
        problem: Arc<Problem>,
        solution: (Arc<Solution>, Option<Cost>),
        random: Arc<dyn Random + Send + Sync>,
    ) -> Self {
        let jobs: Vec<Arc<Job>> = solution.0.unassigned.iter().map(|(job, _)| job.clone()).collect();
        let mut registry = solution.0.registry.deep_copy();
        let mut routes: Vec<RouteContext> = Vec::new();

        solution.0.routes.iter().for_each(|route| {
            if route.tour.has_jobs() {
                let mut route_ctx = RouteContext {
                    route: Arc::new(RwLock::new(route.deep_copy())),
                    state: Arc::new(RwLock::new(RouteState::new())),
                };
                problem.constraint.accept_route_state(&mut route_ctx);
                routes.push(route_ctx);
            } else {
                registry.free_actor(&route.actor);
            }
        });

        InsertionContext {
            progress: InsertionProgress {
                cost: solution.1,
                completeness: 1. - (solution.0.unassigned.len() as f64 / problem.jobs.size() as f64),
                total: problem.jobs.size(),
            },
            problem: problem.clone(),
            solution: SolutionContext {
                required: jobs,
                ignored: vec![],
                unassigned: Default::default(),
                routes,
                registry,
            },
            locked: Self::get_locked_jobs(&problem),
            random: random.clone(),
        }
    }

    /// Restores valid context state.
    pub fn restore(&mut self) {
        self.remove_empty_routes();

        let constraint = self.problem.constraint.clone();
        self.solution.routes.iter_mut().for_each(|route_ctx| {
            constraint.accept_route_state(route_ctx);
        });
    }

    pub fn deep_copy(&self) -> Self {
        InsertionContext {
            progress: self.progress.clone(),
            problem: self.problem.clone(),
            solution: self.solution.deep_copy(),
            locked: self.locked.clone(),
            random: self.random.clone(),
        }
    }

    /// Removes empty routes from solution context.
    fn remove_empty_routes(&mut self) {
        let registry = &mut self.solution.registry;
        self.solution.routes.retain(|rc| {
            let route = rc.route.read().unwrap();
            if route.tour.has_jobs() {
                true
            } else {
                registry.free_actor(&route.actor);
                false
            }
        });
    }

    fn get_locked_jobs(problem: &Problem) -> Arc<HashSet<Arc<Job>>> {
        Arc::new(problem.locks.iter().fold(HashSet::new(), |mut acc, lock| {
            acc.extend(lock.details.iter().flat_map(|d| d.jobs.iter().cloned()));
            acc
        }))
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

    /// Set of routes within their state.
    pub routes: Vec<RouteContext>,

    /// Keeps track of used resources.
    pub registry: Registry,
}

impl SolutionContext {
    pub fn to_solution(&self, extras: Arc<Extras>) -> Solution {
        Solution {
            registry: self.registry.deep_copy(),
            routes: self.routes.iter().map(|rc| rc.route.read().unwrap().deep_copy()).collect(),
            unassigned: self.unassigned.clone(),
            extras,
        }
    }

    pub fn deep_copy(&self) -> Self {
        Self {
            required: self.required.clone(),
            ignored: self.ignored.clone(),
            unassigned: self.unassigned.clone(),
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

impl RouteContext {
    pub fn new(actor: Arc<Actor>) -> Self {
        let mut tour = Tour::new();
        tour.set_start(create_start_activity(&actor));
        create_end_activity(&actor).map(|end| tour.set_end(end));

        RouteContext {
            route: Arc::new(RwLock::new(Route { actor, tour })),
            state: Arc::new(RwLock::new(RouteState::new())),
        }
    }

    pub fn deep_copy(&self) -> Self {
        let orig_route = self.route.read().unwrap();
        let orig_state = self.state.read().unwrap();

        let new_route = Route { actor: orig_route.actor.clone(), tour: orig_route.tour.deep_copy() };
        let mut new_state = RouteState::new_with_sizes(orig_state.sizes());

        // copy activity states
        orig_route.tour.all_activities().zip(0usize..).for_each(|(a, index)| {
            orig_state.all_keys().for_each(|key| {
                if let Some(value) = orig_state.get_activity_state_raw(key, a) {
                    let a = new_route.tour.get(index).unwrap();
                    new_state.put_activity_state_raw(key, a, value.clone());
                }
            });
        });

        // copy route states
        orig_state.all_keys().for_each(|key| {
            if let Some(value) = orig_state.get_route_state_raw(key) {
                new_state.put_route_state_raw(key, value.clone());
            }
        });

        RouteContext { route: Arc::new(RwLock::new(new_route)), state: Arc::new(RwLock::new(new_state)) }
    }
}

pub fn create_start_activity(actor: &Arc<Actor>) -> TourActivity {
    Box::new(Activity {
        place: Place {
            location: actor.detail.start.unwrap_or_else(|| unimplemented!("Optional start is not yet implemented")),
            duration: 0.0,
            time: actor.detail.time.clone(),
        },
        schedule: Schedule { arrival: actor.detail.time.start, departure: actor.detail.time.start },
        job: None,
    })
}

pub fn create_end_activity(actor: &Arc<Actor>) -> Option<TourActivity> {
    actor.detail.end.map(|location| {
        Box::new(Activity {
            place: Place { location, duration: 0.0, time: actor.detail.time.clone() },
            schedule: Schedule { arrival: actor.detail.time.end, departure: actor.detail.time.end },
            job: None,
        })
    })
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        self.route.read().unwrap().deref() as *const Route == other.route.read().unwrap().deref() as *const Route
    }
}

impl Eq for RouteContext {}
