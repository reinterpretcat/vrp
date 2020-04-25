#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/context_test.rs"]
mod context_test;

use crate::construction::constraints::{TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::construction::heuristics::factories::*;
use crate::construction::OP_START_MSG;
use crate::models::common::{Cost, Schedule};
use crate::models::problem::*;
use crate::models::solution::*;
use crate::models::{Extras, Problem, Solution};
use crate::utils::{as_mut, compare_floats, Random};
use hashbrown::{HashMap, HashSet};
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

/// A context which contains information needed for heuristic and metaheuristic.
pub struct InsertionContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Solution context: discovered solution.
    pub solution: SolutionContext,

    /// Random generator.
    pub random: Arc<dyn Random + Send + Sync>,
}

impl InsertionContext {
    /// Creates insertion context from existing solution.
    pub fn new(problem: Arc<Problem>, random: Arc<dyn Random + Send + Sync>) -> Self {
        create_insertion_context(problem, random)
    }

    /// Creates insertion context from existing solution.
    pub fn new_from_solution(
        problem: Arc<Problem>,
        solution: (Arc<Solution>, Option<Cost>),
        random: Arc<dyn Random + Send + Sync>,
    ) -> Self {
        create_insertion_context_from_solution(problem, solution, random)
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

/// Contains information regarding discovered solution.
pub struct SolutionContext {
    /// List of jobs which require permanent assignment.
    pub required: Vec<Job>,

    /// List of jobs which at the moment does not require assignment and might be ignored.
    pub ignored: Vec<Job>,

    /// Map of jobs which cannot be assigned and within reason code.
    pub unassigned: HashMap<Job, i32>,

    /// Specifies jobs which should not be affected by ruin.
    pub locked: HashSet<Job>,

    /// Set of routes within their state.
    pub routes: Vec<RouteContext>,

    /// Keeps track of used resources.
    pub registry: Registry,
}

impl SolutionContext {
    pub fn get_actual_cost(&self) -> Cost {
        self.routes.iter().fold(Cost::default(), |acc, rc| acc + Self::get_route_cost(rc))
    }

    pub fn get_max_cost(&self) -> Cost {
        self.routes.iter().map(Self::get_route_cost).max_by(|&a, &b| compare_floats(a, b)).unwrap_or(0.)
    }

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

    fn get_route_cost(route_ctx: &RouteContext) -> Cost {
        let get_cost = |costs: &Costs, distance: f64, duration: f64| {
            costs.fixed
                + costs.per_distance * distance
                + costs.per_driving_time.max(costs.per_service_time).max(costs.per_waiting_time) * duration
        };

        let actor = &route_ctx.route.actor;
        let distance = route_ctx.state.get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
        let duration = route_ctx.state.get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

        get_cost(&actor.vehicle.costs, distance, duration) + get_cost(&actor.driver.costs, distance, duration)
    }
}

pub type RouteStateValue = Arc<dyn Any + Send + Sync>;

/// Specifies insertion context for route.
#[derive(Clone)]
pub struct RouteContext {
    /// Used route.
    pub route: Arc<Route>,

    /// Insertion state.
    pub state: Arc<RouteState>,
}

/// Provides the way to associate arbitrary data within route and activity.
pub struct RouteState {
    route_states: HashMap<i32, RouteStateValue>,
    activity_states: HashMap<ActivityWithKey, RouteStateValue>,
    keys: HashSet<i32>,
}

impl RouteContext {
    pub fn as_mut(&mut self) -> (&mut Route, &mut RouteState) {
        let route: &mut Route = unsafe { as_mut(&self.route) };
        let state: &mut RouteState = unsafe { as_mut(&self.state) };

        (route, state)
    }

    pub fn route_mut(&mut self) -> &mut Route {
        unsafe { as_mut(&self.route) }
    }

    pub fn state_mut(&mut self) -> &mut RouteState {
        unsafe { as_mut(&self.state) }
    }
}

impl RouteContext {
    pub fn new(actor: Arc<Actor>) -> Self {
        let mut tour = Tour::default();
        tour.set_start(create_start_activity(&actor));
        create_end_activity(&actor).map(|end| tour.set_end(end));

        RouteContext { route: Arc::new(Route { actor, tour }), state: Arc::new(RouteState::default()) }
    }

    pub fn deep_copy(&self) -> Self {
        let new_route = Route { actor: self.route.actor.clone(), tour: self.route.tour.deep_copy() };
        let mut new_state = RouteState::new_with_sizes(self.state.sizes());

        // copy activity states
        self.route.tour.all_activities().zip(0_usize..).for_each(|(a, index)| {
            self.state.all_keys().for_each(|key| {
                if let Some(value) = self.state.get_activity_state_raw(key, a) {
                    let a = new_route.tour.get(index).unwrap();
                    new_state.put_activity_state_raw(key, a, value.clone());
                }
            });
        });

        // copy route states
        self.state.all_keys().for_each(|key| {
            if let Some(value) = self.state.get_route_state_raw(key) {
                new_state.put_route_state_raw(key, value.clone());
            }
        });

        RouteContext { route: Arc::new(new_route), state: Arc::new(new_state) }
    }
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        self.route.deref() as *const Route == other.route.deref() as *const Route
    }
}

impl Eq for RouteContext {}

impl Default for RouteState {
    fn default() -> RouteState {
        RouteState::new_with_sizes((2, 4))
    }
}

impl RouteState {
    /// Creates a new RouteState using giving capacities.
    pub fn new_with_sizes(sizes: (usize, usize)) -> RouteState {
        RouteState {
            route_states: HashMap::with_capacity(sizes.0),
            activity_states: HashMap::with_capacity(sizes.1),
            keys: Default::default(),
        }
    }

    /// Gets value associated with key converted to given type.
    pub fn get_route_state<T: Send + Sync + 'static>(&self, key: i32) -> Option<&T> {
        self.route_states.get(&key).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_route_state_raw(&self, key: i32) -> Option<&RouteStateValue> {
        self.route_states.get(&key)
    }

    /// Gets value associated with key converted to given type.
    pub fn get_activity_state<T: Send + Sync + 'static>(&self, key: i32, activity: &TourActivity) -> Option<&T> {
        self.activity_states
            .get(&(activity.as_ref() as *const Activity as usize, key))
            .and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_activity_state_raw(&self, key: i32, activity: &TourActivity) -> Option<&RouteStateValue> {
        self.activity_states.get(&(activity.as_ref() as *const Activity as usize, key))
    }

    /// Puts value associated with key.
    pub fn put_route_state<T: Send + Sync + 'static>(&mut self, key: i32, value: T) {
        self.route_states.insert(key, Arc::new(value));
        self.keys.insert(key);
    }

    /// Puts value associated with key.
    pub fn put_route_state_raw(&mut self, key: i32, value: Arc<dyn Any + Send + Sync>) {
        self.route_states.insert(key, value);
        self.keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state<T: Send + Sync + 'static>(&mut self, key: i32, activity: &TourActivity, value: T) {
        self.activity_states.insert((activity.as_ref() as *const Activity as usize, key), Arc::new(value));
        self.keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state_raw(&mut self, key: i32, activity: &TourActivity, value: RouteStateValue) {
        self.activity_states.insert((activity.as_ref() as *const Activity as usize, key), value);
        self.keys.insert(key);
    }

    /// Removes all activity states for given activity.
    pub fn remove_activity_states(&mut self, activity: &TourActivity) {
        for (_, key) in self.keys.iter().enumerate() {
            self.activity_states.remove(&(activity.as_ref() as *const Activity as usize, *key));
        }
    }

    /// Returns all state keys.
    pub fn all_keys<'a>(&'a self) -> impl Iterator<Item = i32> + 'a {
        self.keys.iter().cloned()
    }

    /// Returns size route state storage.
    pub fn sizes(&self) -> (usize, usize) {
        (self.route_states.capacity(), self.activity_states.capacity())
    }
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

type ActivityWithKey = (usize, i32);
type ActivityPlace = crate::models::solution::Place;

/// Creates start activity.
pub fn create_start_activity(actor: &Arc<Actor>) -> TourActivity {
    Box::new(Activity {
        place: ActivityPlace {
            location: actor.detail.start.unwrap_or_else(|| unimplemented!("{}", OP_START_MSG)),
            duration: 0.0,
            time: actor.detail.time.clone(),
        },
        schedule: Schedule { arrival: actor.detail.time.start, departure: actor.detail.time.start },
        job: None,
    })
}

/// Creates end activity if it is specified for the actor.
pub fn create_end_activity(actor: &Arc<Actor>) -> Option<TourActivity> {
    actor.detail.end.map(|location| {
        Box::new(Activity {
            place: ActivityPlace { location, duration: 0.0, time: actor.detail.time.clone() },
            schedule: Schedule { arrival: actor.detail.time.end, departure: actor.detail.time.end },
            job: None,
        })
    })
}
