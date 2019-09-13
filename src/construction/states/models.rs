use super::*;
use crate::construction::states::route::RouteState;
use crate::models::common::{Cost, Schedule, TimeWindow};
use crate::models::problem::Job;
use crate::models::solution::{Activity, Actor, Place, Registry, Route, Tour, TourActivity};
use crate::models::{Problem, Solution};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
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
pub struct InsertionProgress {
    /// Specifies best known cost depending on context.
    pub cost: Cost,

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
    pub solution: Arc<SolutionContext>,

    /// Random generator.
    pub random: Arc<String>,
}

/// Contains information regarding insertion solution.
pub struct SolutionContext {
    /// List of jobs which require permanent assignment.
    pub required: Vec<Arc<Job>>,

    /// List of jobs which at the moment does not require assignment and might be ignored.
    pub ignored: Vec<Arc<Job>>,

    /// Map of jobs which cannot be assigned and within reason code.
    pub unassigned: HashMap<Arc<Job>, i32>,

    // TODO implement proper hash function for RouteContext
    /// Set of routes within their state.
    pub routes: HashSet<RouteContext>,

    /// Keeps track of used resources.
    pub registry: Arc<Registry>,
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
}

pub fn create_start_activity(actor: &Arc<Actor>) -> TourActivity {
    Box::new(Activity {
        place: Place {
            location: actor.detail.start.unwrap_or_else(|| unimplemented!("Optional start is not yet implemented")),
            duration: 0.0,
            time: TimeWindow { start: actor.detail.time.start, end: std::f64::MAX },
        },
        schedule: Schedule { arrival: actor.detail.time.start, departure: actor.detail.time.start },
        job: None,
    })
}

pub fn create_end_activity(actor: &Arc<Actor>) -> Option<TourActivity> {
    actor.detail.end.map(|location| {
        Box::new(Activity {
            place: Place { location, duration: 0.0, time: TimeWindow { start: 0.0, end: actor.detail.time.end } },
            schedule: Schedule { arrival: actor.detail.time.end, departure: actor.detail.time.end },
            job: None,
        })
    })
}

impl Hash for RouteContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = self.route.read().unwrap().deref() as *const Route;
        ptr.hash(state);
    }
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        self.route.read().unwrap().deref() as *const Route == other.route.read().unwrap().deref() as *const Route
    }
}

impl Eq for RouteContext {}
