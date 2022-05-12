#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/context_test.rs"]
mod context_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::factories::*;
use crate::models::common::Cost;
use crate::models::problem::*;
use crate::models::solution::*;
use crate::models::{Extras, Problem, Solution};
use crate::utils::as_mut;
use hashbrown::{HashMap, HashSet};
use rosomaxa::prelude::*;
use std::any::Any;
use std::ops::Deref;
use std::sync::Arc;

/// A context which contains information needed for heuristic and metaheuristic.
pub struct InsertionContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Solution context: discovered solution.
    pub solution: SolutionContext,

    /// Information about environment.
    pub environment: Arc<Environment>,
}

impl InsertionContext {
    /// Creates insertion context for given problem with unassigned jobs.
    pub fn new(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        create_insertion_context(problem, environment)
    }

    /// Creates insertion context for given problem with empty solution.
    pub fn new_empty(problem: Arc<Problem>, environment: Arc<Environment>) -> Self {
        create_empty_insertion_context(problem, environment)
    }

    /// Creates insertion context from existing solution.
    pub fn new_from_solution(
        problem: Arc<Problem>,
        solution: (Solution, Option<Cost>),
        environment: Arc<Environment>,
    ) -> Self {
        let mut ctx = create_insertion_context_from_solution(problem, solution, environment);
        ctx.restore();

        ctx
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

    /// Removes empty routes from solution context.
    fn remove_empty_routes(&mut self) {
        let registry = &mut self.solution.registry;
        self.solution.routes.retain(|rc| {
            if rc.route.tour.has_jobs() {
                true
            } else {
                registry.free_route(rc);
                false
            }
        });
    }
}

impl HeuristicSolution for InsertionContext {
    fn get_fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.problem.objective.objectives().map(move |objective| objective.fitness(self)))
    }

    fn deep_copy(&self) -> Self {
        InsertionContext {
            problem: self.problem.clone(),
            solution: self.solution.deep_copy(),
            environment: self.environment.clone(),
        }
    }
}

/// A any state value.
pub type StateValue = Arc<dyn Any + Send + Sync>;

/// Keeps information about unassigned reason code.
#[derive(Clone)]
pub enum UnassignedCode {
    /// No code is available.
    Unknown,
    /// Only single code is available.
    Simple(i32),
    /// A collection of actor-code pairs is available.
    Detailed(Vec<(Arc<Actor>, i32)>),
}

/// Contains information regarding discovered solution.
pub struct SolutionContext {
    /// List of jobs which require permanent assignment.
    pub required: Vec<Job>,

    /// List of jobs which at the moment does not require assignment and might be ignored.
    pub ignored: Vec<Job>,

    /// Map of jobs which cannot be assigned and within reason code.
    pub unassigned: HashMap<Job, UnassignedCode>,

    /// Specifies jobs which should not be affected by ruin.
    pub locked: HashSet<Job>,

    /// Set of routes within their state.
    pub routes: Vec<RouteContext>,

    /// Keeps track of used routes and resources.
    pub registry: RegistryContext,

    /// A collection of data associated with solution.
    pub state: HashMap<i32, StateValue>,
}

impl SolutionContext {
    /// Gets total cost of the solution.
    pub fn get_total_cost(&self) -> Cost {
        self.routes.iter().fold(Cost::default(), |acc, rc| acc + rc.get_route_cost())
    }

    /// Gets the most expensive route cost.
    pub fn get_max_cost(&self) -> Cost {
        self.routes.iter().map(|rc| rc.get_route_cost()).max_by(|&a, &b| compare_floats(a, b)).unwrap_or(0.)
    }

    /// Converts given `SolutionContext` to Solution model.
    pub fn to_solution(&self, extras: Arc<Extras>) -> Solution {
        Solution {
            registry: self.registry.resources().deep_copy(),
            routes: self.routes.iter().map(|rc| rc.route.deep_copy()).collect(),
            unassigned: self
                .unassigned
                .iter()
                .map(|(job, code)| (job.clone(), code.clone()))
                .chain(self.required.iter().map(|job| (job.clone(), UnassignedCode::Unknown)))
                .collect(),
            extras,
        }
    }

    /// Creates a deep copy of `SolutionContext`.
    pub fn deep_copy(&self) -> Self {
        Self {
            required: self.required.clone(),
            ignored: self.ignored.clone(),
            unassigned: self.unassigned.clone(),
            locked: self.locked.clone(),
            routes: self.routes.iter().map(|rc| rc.deep_copy()).collect(),
            registry: self.registry.deep_copy(),
            state: self.state.clone(),
        }
    }
}

/// Specifies insertion context for route.
#[derive(Clone)]
pub struct RouteContext {
    /// Used route.
    pub route: Arc<Route>,

    /// Insertion state.
    pub state: Arc<RouteState>,

    /// A route cache.
    cache: Arc<RouteCache>,
}

/// Provides the way to associate arbitrary data within route or/and activity.
/// NOTE: do not put any state which is not refreshed after accept_route_state call: it will be
/// wiped out at some point.
pub struct RouteState {
    route_states: HashMap<i32, StateValue>,
    activity_states: HashMap<ActivityWithKey, StateValue>,
    route_keys: HashSet<i32>,
    activity_keys: HashSet<i32>,
    flags: u8,
}

/// Specifies route state flags which are stateful and not reset during `accept_route_state`.
pub mod state_flags {
    /// No flags set.
    pub const NO_FLAGS: u8 = 0x00;
    /// Route is in unassignable state.
    pub const UNASSIGNABLE: u8 = 0x01;
}

impl RouteContext {
    /// Creates a new instance of `RouteContext`.
    pub fn new(actor: Arc<Actor>) -> Self {
        let tour = Tour::new(&actor);
        Self::new_with_state(Arc::new(Route { actor, tour }), Arc::new(RouteState::default()))
    }

    /// Creates a new instance of `RouteContext` with arguments provided.
    pub fn new_with_state(route: Arc<Route>, state: Arc<RouteState>) -> Self {
        RouteContext { route, state, cache: Arc::new(RouteCache { is_stale: true }) }
    }

    /// Creates a deep copy of `RouteContext`.
    pub fn deep_copy(&self) -> Self {
        let new_route = Route { actor: self.route.actor.clone(), tour: self.route.tour.deep_copy() };
        let new_state = RouteState::from_other_and_tours(self.state.as_ref(), &self.route.tour, &new_route.tour);

        RouteContext {
            route: Arc::new(new_route),
            state: Arc::new(new_state),
            cache: Arc::new(RouteCache { is_stale: self.cache.is_stale }),
        }
    }

    /// Gets route cost.
    pub fn get_route_cost(&self) -> Cost {
        let get_cost = |costs: &Costs, distance: f64, duration: f64| {
            costs.fixed
                + costs.per_distance * distance
                // NOTE this is incorrect when timing costs are different: fitness value will be
                // different from actual cost. However we accept this so far as it is simpler for
                // implementation and pragmatic format does not expose this feature
                // .
                // TODO calculate actual cost
                + costs.per_driving_time.max(costs.per_service_time).max(costs.per_waiting_time) * duration
        };

        let actor = &self.route.actor;
        let distance = self.state.get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
        let duration = self.state.get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

        get_cost(&actor.vehicle.costs, distance, duration) + get_cost(&actor.driver.costs, distance, duration)
    }

    /// Unwraps given `RouteContext` as pair of mutable references.
    /// Marks context as stale.
    pub fn as_mut(&mut self) -> (&mut Route, &mut RouteState) {
        self.mark_stale(true);

        let route: &mut Route = unsafe { as_mut(&self.route) };
        let state: &mut RouteState = unsafe { as_mut(&self.state) };

        (route, state)
    }

    /// Returns mutable reference to used `Route`.
    /// Marks context as stale.
    pub fn route_mut(&mut self) -> &mut Route {
        self.mark_stale(true);
        unsafe { as_mut(&self.route) }
    }

    /// Returns mutable reference to used `RouteState`.
    /// Marks context as stale.
    pub fn state_mut(&mut self) -> &mut RouteState {
        self.mark_stale(true);
        unsafe { as_mut(&self.state) }
    }

    /// Returns true if context is stale. Context is marked stale when it is accessed by `mut`
    /// methods. A general motivation of the flag is to avoid recalculating non-changed states.
    pub fn is_stale(&self) -> bool {
        self.cache.is_stale
    }

    /// Marks context stale or resets the flag.
    pub(crate) fn mark_stale(&mut self, is_stale: bool) {
        let cache: &mut RouteCache = unsafe { as_mut(&self.cache) };
        cache.is_stale = is_stale;
    }
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        std::ptr::eq(self.route.deref(), other.route.deref())
    }
}

impl Eq for RouteContext {}

impl Default for RouteState {
    fn default() -> RouteState {
        RouteState {
            route_states: HashMap::with_capacity(2),
            activity_states: HashMap::with_capacity(4),
            route_keys: HashSet::with_capacity(2),
            activity_keys: HashSet::with_capacity(4),
            flags: state_flags::NO_FLAGS,
        }
    }
}

impl RouteState {
    /// A fast way to create `RouteState`.
    pub(crate) fn from_other_and_tours(other: &Self, old_tour: &Tour, new_tour: &Tour) -> Self {
        let route_states = other.route_states.clone();
        let route_keys = other.route_keys.clone();
        let activity_keys = other.activity_keys.clone();
        let mut activity_states = HashMap::with_capacity(other.activity_states.len());

        old_tour.all_activities().enumerate().for_each(|(index, activity)| {
            other.all_activity_keys().for_each(|key| {
                if let Some(value) = other.get_activity_state_raw(key, activity) {
                    let activity = new_tour.get(index).unwrap();
                    activity_states.insert((activity as *const Activity as usize, key), value.clone());
                }
            });
        });

        Self { route_states, activity_states, route_keys, activity_keys, flags: other.flags }
    }

    /// Gets value associated with key converted to given type.
    pub fn get_route_state<T: Send + Sync + 'static>(&self, key: i32) -> Option<&T> {
        self.route_states.get(&key).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_route_state_raw(&self, key: i32) -> Option<&StateValue> {
        self.route_states.get(&key)
    }

    /// Gets value associated with key converted to given type.
    pub fn get_activity_state<T: Send + Sync + 'static>(&self, key: i32, activity: &Activity) -> Option<&T> {
        self.activity_states.get(&(activity as *const Activity as usize, key)).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_activity_state_raw(&self, key: i32, activity: &Activity) -> Option<&StateValue> {
        self.activity_states.get(&(activity as *const Activity as usize, key))
    }

    /// Puts value associated with key.
    pub fn put_route_state<T: Send + Sync + 'static>(&mut self, key: i32, value: T) {
        self.route_states.insert(key, Arc::new(value));
        self.route_keys.insert(key);
    }

    /// Puts value associated with key.
    pub fn put_route_state_raw(&mut self, key: i32, value: Arc<dyn Any + Send + Sync>) {
        self.route_states.insert(key, value);
        self.route_keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state<T: Send + Sync + 'static>(&mut self, key: i32, activity: &Activity, value: T) {
        self.activity_states.insert((activity as *const Activity as usize, key), Arc::new(value));
        self.activity_keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state_raw(&mut self, key: i32, activity: &Activity, value: StateValue) {
        self.activity_states.insert((activity as *const Activity as usize, key), value);
        self.activity_keys.insert(key);
    }

    /// Removes all activity states for given activity.
    pub fn remove_activity_states(&mut self, activity: &Activity) {
        for (_, key) in self.activity_keys.iter().enumerate() {
            self.activity_states.remove(&(activity as *const Activity as usize, *key));
        }
    }

    /// Returns all activity state keys.
    pub fn all_activity_keys(&'_ self) -> impl Iterator<Item = i32> + '_ {
        self.activity_keys.iter().cloned()
    }

    /// Returns all route state keys.
    pub fn all_route_keys(&'_ self) -> impl Iterator<Item = i32> + '_ {
        self.route_keys.iter().cloned()
    }

    /// Returns size route state storage.
    pub fn sizes(&self) -> (usize, usize) {
        (self.route_states.capacity(), self.activity_states.capacity())
    }

    /// Sets flag.
    pub fn set_flag(&mut self, flag: u8) {
        self.flags |= flag;
    }

    /// Gets flags.
    pub fn get_flags(&self) -> u8 {
        self.flags
    }

    /// Returns true if flag is set.
    pub fn has_flag(&self, flag: u8) -> bool {
        (self.flags & flag) > 0
    }

    /// Resets all flags.
    pub fn reset_flags(&mut self) {
        self.flags = state_flags::NO_FLAGS
    }

    /// Clear all states, but keeps flags.
    pub fn clear(&mut self) {
        self.activity_keys.clear();
        self.activity_states.clear();

        self.route_keys.clear();
        self.route_states.clear();
    }
}

struct RouteCache {
    is_stale: bool,
}

/// A wrapper around route context modifier function.
pub struct RouteModifier {
    modifier: Arc<dyn Fn(RouteContext) -> RouteContext + Sync + Send>,
}

impl RouteModifier {
    /// Creates a new instance of `RouteModifier`.
    pub fn new<F: 'static + Fn(RouteContext) -> RouteContext + Sync + Send>(modifier: F) -> Self {
        Self { modifier: Arc::new(modifier) }
    }

    /// Modifies route context if necessary.
    pub fn modify(&self, route_ctx: RouteContext) -> RouteContext {
        self.modifier.deref()(route_ctx)
    }
}

/// Keeps track on how routes are used.
pub struct RegistryContext {
    registry: Registry,
    index: HashMap<Arc<Actor>, RouteContext>,
}

impl RegistryContext {
    /// Creates a new instance of `RouteRegistry`.
    pub fn new(constraint: Arc<ConstraintPipeline>, registry: Registry) -> Self {
        Self::new_with_modifier(constraint, registry, &RouteModifier::new(move |route_ctx| route_ctx))
    }

    /// Creates a new instance of `RouteRegistry` using route context modifier.
    pub fn new_with_modifier(
        constraint: Arc<ConstraintPipeline>,
        registry: Registry,
        modifier: &RouteModifier,
    ) -> Self {
        let index = registry
            .all()
            .map(|actor| {
                let mut route_ctx = RouteContext::new(actor.clone());
                constraint.accept_route_state(&mut route_ctx);

                (actor, modifier.modify(route_ctx))
            })
            .collect();

        Self { registry, index }
    }

    /// Returns underlying registry.
    pub fn resources(&self) -> &Registry {
        &self.registry
    }

    /// Returns next route for insertion.
    pub fn next(&'_ self) -> impl Iterator<Item = RouteContext> + '_ {
        self.registry.next().map(move |actor| self.index[&actor].clone())
    }

    /// Returns route for given actor if it is available.
    pub fn next_with_actor(&self, actor: &Actor) -> Option<RouteContext> {
        self.registry.available().find(|a| actor == a.as_ref()).and_then(|a| self.index.get(&a).cloned())
    }

    /// Sets this route as used.
    /// Returns whether the route was already marked as used in the registry.
    pub fn use_route(&mut self, route: &RouteContext) -> bool {
        self.registry.use_actor(&route.route.actor)
    }

    /// Sets this route as unused.
    /// Returns whether the route was already unused in the registry.
    pub fn free_route(&mut self, route: &RouteContext) {
        self.registry.free_actor(&route.route.actor);
    }

    /// Creates a deep copy of `RegistryContext`.
    pub fn deep_copy(&self) -> Self {
        Self { registry: self.registry.deep_copy(), index: self.index.clone() }
    }

    /// Creates a deep sliced copy of RegistryContext` keeping only specific actors data.
    pub fn deep_slice(&self, filter: impl Fn(&Actor) -> bool) -> Self {
        let index = self
            .index
            .iter()
            .filter(|(actor, _)| filter(actor.as_ref()))
            .map(|(actor, route_ctx)| (actor.clone(), route_ctx.clone()))
            .collect();
        Self { registry: self.registry.deep_slice(filter), index }
    }
}

/// Specifies insertion context for activity.
pub struct ActivityContext<'a> {
    /// Activity insertion index.
    pub index: usize,

    /// Previous activity.
    pub prev: &'a Activity,

    /// Target activity.
    pub target: &'a Activity,

    /// Next activity. Absent if tour is open and target activity inserted last.
    pub next: Option<&'a Activity>,
}

type ActivityWithKey = (usize, i32);
