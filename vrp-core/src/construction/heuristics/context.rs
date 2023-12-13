#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/context_test.rs"]
mod context_test;

use crate::construction::features::{TOTAL_DISTANCE_KEY, TOTAL_DURATION_KEY};
use crate::construction::heuristics::factories::*;
use crate::models::common::Cost;
use crate::models::problem::*;
use crate::models::solution::*;
use crate::models::GoalContext;
use crate::models::{Problem, Solution};
use hashbrown::{HashMap, HashSet};
use nohash_hasher::BuildNoHashHasher;
use rosomaxa::evolution::TelemetryMetrics;
use rosomaxa::prelude::*;
use rustc_hash::FxHasher;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::hash::BuildHasherDefault;
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
        self.problem.goal.accept_solution_state(&mut self.solution);
        self.solution.remove_empty_routes();
    }
}

impl HeuristicSolution for InsertionContext {
    fn fitness<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        self.problem.goal.fitness(self)
    }

    fn deep_copy(&self) -> Self {
        InsertionContext {
            problem: self.problem.clone(),
            solution: self.solution.deep_copy(),
            environment: self.environment.clone(),
        }
    }
}

impl Debug for InsertionContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("problem", &self.problem)
            .field("solution", &self.solution)
            .finish_non_exhaustive()
    }
}

/// A any state value.
pub type StateValue = Arc<dyn Any + Send + Sync>;

/// Keeps information about unassigned reason code.
#[derive(Clone, Debug)]
pub enum UnassignmentInfo {
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
    pub unassigned: HashMap<Job, UnassignmentInfo>,

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

        self.routes.iter().fold(Cost::default(), |acc, route_ctx| {
            let actor = &route_ctx.route.actor;
            let distance = route_ctx.state.get_route_state::<f64>(TOTAL_DISTANCE_KEY).cloned().unwrap_or(0.);
            let duration = route_ctx.state.get_route_state::<f64>(TOTAL_DURATION_KEY).cloned().unwrap_or(0.);

            acc + get_cost(&actor.vehicle.costs, distance, duration) + get_cost(&actor.driver.costs, distance, duration)
        })
    }

    /// Returns amount of jobs considered by solution context.
    /// NOTE: the amount can be different for partially solved problem from original problem.
    pub fn get_jobs_amount(&self) -> usize {
        let assigned = self.routes.iter().map(|route_ctx| route_ctx.route().tour.job_count()).sum::<usize>();

        let required = self.required.iter().filter(|job| !self.unassigned.contains_key(*job)).count();

        self.unassigned.len() + required + self.ignored.len() + assigned
    }

    /// Keep routes for which given predicate returns true.
    pub fn keep_routes(&mut self, predicate: &dyn Fn(&RouteContext) -> bool) {
        // as for 1.68, drain_filter is not yet stable (see https://github.com/rust-lang/rust/issues/43244)
        let (keep, remove): (Vec<_>, Vec<_>) = self.routes.drain(0..).partition(predicate);

        remove.into_iter().for_each(|route_ctx| {
            assert!(self.registry.free_route(route_ctx));
        });

        self.routes = keep;
    }

    /// Removes empty routes from solution context.
    pub(crate) fn remove_empty_routes(&mut self) {
        self.keep_routes(&|route_ctx| route_ctx.route().tour.has_jobs())
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

impl Debug for SolutionContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("required", &self.required.len())
            .field("locked", &self.locked.len())
            .field("routes", &self.routes)
            .field("unassigned", &self.unassigned)
            .finish_non_exhaustive()
    }
}

impl From<SolutionContext> for Solution {
    fn from(solution_ctx: SolutionContext) -> Self {
        (solution_ctx, None).into()
    }
}

impl From<(SolutionContext, Option<TelemetryMetrics>)> for Solution {
    fn from(value: (SolutionContext, Option<TelemetryMetrics>)) -> Self {
        let (solution_ctx, telemetry) = value;
        Solution {
            cost: solution_ctx.get_total_cost(),
            registry: solution_ctx.registry.resources().deep_copy(),
            routes: solution_ctx.routes.iter().map(|rc| rc.route.deep_copy()).collect(),
            unassigned: solution_ctx
                .unassigned
                .iter()
                .map(|(job, code)| (job.clone(), code.clone()))
                .chain(solution_ctx.required.iter().map(|job| (job.clone(), UnassignmentInfo::Unknown)))
                .collect(),
            telemetry,
        }
    }
}

/// Specifies insertion context for route.
pub struct RouteContext {
    route: Route,
    state: RouteState,
    cache: RouteCache,
}

/// Provides the way to associate arbitrary data within route or/and activity.
/// NOTE: do not put any state which is not refreshed after `accept_route_state` call: it will be
/// wiped out at some point.
pub struct RouteState {
    route_states: HashMap<i32, StateValue, BuildNoHashHasher<i32>>,
    activity_states: HashMap<ActivityWithKey, StateValue, BuildHasherDefault<FxHasher>>,
    route_keys: HashSet<i32, BuildNoHashHasher<i32>>,
    activity_keys: HashSet<i32, BuildNoHashHasher<i32>>,
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
        Self::new_with_state(Route { actor, tour }, RouteState::default())
    }

    /// Creates a new instance of `RouteContext` with arguments provided.
    pub fn new_with_state(route: Route, state: RouteState) -> Self {
        RouteContext { route, state, cache: RouteCache { is_stale: true } }
    }

    /// Creates a deep copy of `RouteContext`.
    pub fn deep_copy(&self) -> Self {
        let new_route = Route { actor: self.route.actor.clone(), tour: self.route.tour.deep_copy() };
        let new_state = RouteState::from_other(&self.state, self.route.tour.total());

        RouteContext { route: new_route, state: new_state, cache: RouteCache { is_stale: self.cache.is_stale } }
    }

    /// Returns a reference to route.
    pub fn route(&self) -> &Route {
        &self.route
    }

    /// Returns a reference to state.
    pub fn state(&self) -> &RouteState {
        &self.state
    }

    /// Unwraps given `RouteContext` as pair of mutable references.
    /// Marks context as stale.
    pub fn as_mut(&mut self) -> (&mut Route, &mut RouteState) {
        self.mark_stale(true);
        (&mut self.route, &mut self.state)
    }

    /// Returns mutable reference to used `Route`.
    /// Marks context as stale.
    pub fn route_mut(&mut self) -> &mut Route {
        self.mark_stale(true);
        &mut self.route
    }

    /// Returns mutable reference to used `RouteState`.
    /// Marks context as stale.
    pub fn state_mut(&mut self) -> &mut RouteState {
        self.mark_stale(true);
        &mut self.state
    }

    /// Returns true if context is stale. Context is marked stale when it is accessed by `mut`
    /// methods. A general motivation of the flag is to avoid recalculating non-changed states.
    pub fn is_stale(&self) -> bool {
        self.cache.is_stale
    }

    /// Marks context stale or resets the flag.
    pub(crate) fn mark_stale(&mut self, is_stale: bool) {
        self.cache.is_stale = is_stale;
    }
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        std::ptr::eq(self.route.actor.deref(), other.route.actor.deref())
    }
}

impl Eq for RouteContext {}

impl Debug for RouteContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("route", &self.route)
            .field("is_stale", &self.is_stale())
            .finish_non_exhaustive()
    }
}

impl Default for RouteState {
    fn default() -> RouteState {
        RouteState {
            route_states: HashMap::with_capacity_and_hasher(2, BuildNoHashHasher::<i32>::default()),
            activity_states: HashMap::with_capacity_and_hasher(4, BuildHasherDefault::<FxHasher>::default()),
            route_keys: HashSet::with_capacity_and_hasher(2, BuildNoHashHasher::<i32>::default()),
            activity_keys: HashSet::with_capacity_and_hasher(4, BuildNoHashHasher::<i32>::default()),
            flags: state_flags::NO_FLAGS,
        }
    }
}

impl RouteState {
    /// A fast way to create `RouteState`.
    pub(crate) fn from_other(other: &Self, total_activities: usize) -> Self {
        let route_states = other.route_states.clone();
        let route_keys = other.route_keys.clone();
        let activity_keys = other.activity_keys.clone();
        let mut activity_states =
            HashMap::with_capacity_and_hasher(other.activity_states.len(), BuildHasherDefault::<FxHasher>::default());

        (0..total_activities).for_each(|activity_idx| {
            other.all_activity_keys().for_each(|key| {
                if let Some(value) = other.get_activity_state_raw(key, activity_idx) {
                    activity_states.insert((activity_idx, key), value.clone());
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
    pub fn get_activity_state<T: Send + Sync + 'static>(&self, key: i32, activity_idx: usize) -> Option<&T> {
        self.activity_states.get(&(activity_idx, key)).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_activity_state_raw(&self, key: i32, activity_idx: usize) -> Option<&StateValue> {
        self.activity_states.get(&(activity_idx, key))
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
    pub fn put_activity_state<T: Send + Sync + 'static>(&mut self, key: i32, activity_idx: usize, value: T) {
        self.activity_states.insert((activity_idx, key), Arc::new(value));
        self.activity_keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state_raw(&mut self, key: i32, activity_idx: usize, value: StateValue) {
        self.activity_states.insert((activity_idx, key), value);
        self.activity_keys.insert(key);
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
        (self.modifier)(route_ctx)
    }
}

/// Keeps track on how routes are used.
pub struct RegistryContext {
    registry: Registry,
    index: HashMap<Arc<Actor>, RouteContext>,
}

impl RegistryContext {
    /// Creates a new instance of `RouteRegistry`.
    pub fn new(goal: Arc<GoalContext>, registry: Registry) -> Self {
        Self::new_with_modifier(goal, registry, &RouteModifier::new(move |route_ctx| route_ctx))
    }

    /// Creates a new instance of `RouteRegistry` using route context modifier.
    pub fn new_with_modifier(goal: Arc<GoalContext>, registry: Registry, modifier: &RouteModifier) -> Self {
        let index = registry
            .all()
            .map(|actor| {
                let mut route_ctx = RouteContext::new(actor.clone());
                goal.accept_route_state(&mut route_ctx);

                (actor, modifier.modify(route_ctx))
            })
            .collect();

        Self { registry, index }
    }

    /// Returns underlying registry.
    pub fn resources(&self) -> &Registry {
        &self.registry
    }

    /// Returns next route available for insertion.
    pub fn next_route(&self) -> impl Iterator<Item = &RouteContext> {
        self.registry.next().map(move |actor| &self.index[&actor])
    }

    /// Gets route for given actor and marks it as used.
    /// Returns None if actor is already in use.
    /// NOTE: you need to call free route to make it to be available again.
    pub fn get_route(&mut self, actor: &Actor) -> Option<RouteContext> {
        let route_ctx = self
            .registry
            .available()
            .find(|a| actor == a.as_ref())
            .and_then(|a| self.index.get(&a))
            .map(|route_ctx| route_ctx.deep_copy());

        if let Some(route_ctx) = route_ctx {
            assert!(self.registry.use_actor(&route_ctx.route().actor));
            Some(route_ctx)
        } else {
            None
        }
    }

    /// Return back route to be reused again.
    /// Returns whether the route was not present in the registry.
    pub fn free_route(&mut self, route: RouteContext) -> bool {
        self.registry.free_actor(&route.route.actor)
    }

    /// Creates a deep copy of `RegistryContext`.
    pub fn deep_copy(&self) -> Self {
        Self {
            registry: self.registry.deep_copy(),
            index: self.index.iter().map(|(actor, route_ctx)| (actor.clone(), route_ctx.deep_copy())).collect(),
        }
    }

    /// Creates a deep sliced copy of `RegistryContext` keeping only specific actors data.
    pub fn deep_slice(&self, filter: impl Fn(&Actor) -> bool) -> Self {
        let index = self
            .index
            .iter()
            .filter(|(actor, _)| filter(actor.as_ref()))
            .map(|(actor, route_ctx)| (actor.clone(), route_ctx.deep_copy()))
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

/// A local move context.
pub enum MoveContext<'a> {
    /// Evaluation of job insertion into the given route.
    Route {
        /// A solution context.
        solution_ctx: &'a SolutionContext,
        /// A route context where job supposed to be inserted.
        route_ctx: &'a RouteContext,
        /// A job which being evaluated.
        job: &'a Job,
    },
    /// Evaluation of activity insertion into the given position.
    Activity {
        /// A route context where activity supposed to be inserted.
        route_ctx: &'a RouteContext,
        /// An activity context.
        activity_ctx: &'a ActivityContext<'a>,
    },
}

impl<'a> MoveContext<'a> {
    /// Creates a route variant for `MoveContext`.
    pub fn route(solution_ctx: &'a SolutionContext, route_ctx: &'a RouteContext, job: &'a Job) -> MoveContext<'a> {
        MoveContext::Route { solution_ctx, route_ctx, job }
    }

    /// Creates a route variant for `MoveContext`.
    pub fn activity(route_ctx: &'a RouteContext, activity_ctx: &'a ActivityContext) -> MoveContext<'a> {
        MoveContext::Activity { route_ctx, activity_ctx }
    }
}
