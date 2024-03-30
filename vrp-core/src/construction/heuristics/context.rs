#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/context_test.rs"]
mod context_test;

use crate::construction::heuristics::factories::*;
use crate::models::common::Cost;
use crate::models::problem::*;
use crate::models::solution::*;
use crate::models::{CoreStateKeys, GoalContext};
use crate::models::{Problem, Solution};
use hashbrown::{HashMap, HashSet};
use nohash_hasher::BuildNoHashHasher;
use rosomaxa::evolution::TelemetryMetrics;
use rosomaxa::prelude::*;
use std::any::Any;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::sync::Arc;

/// A context which contains information needed for heuristic and metaheuristic.
pub struct InsertionContext {
    /// Original problem.
    pub problem: Arc<Problem>,

    /// Solution context: discovered solution.
    pub solution: SolutionContext,

    /// Information about environment.
    pub environment: DefaultEnvironment,
}

impl InsertionContext {
    /// Creates insertion context for given problem with unassigned jobs.
    pub fn new(problem: Arc<Problem>, environment: DefaultEnvironment) -> Self {
        create_insertion_context(problem, environment)
    }

    /// Creates insertion context for given problem with empty solution.
    pub fn new_empty(problem: Arc<Problem>, environment: DefaultEnvironment) -> Self {
        create_empty_insertion_context(problem, environment)
    }

    /// Creates insertion context from existing solution.
    pub fn new_from_solution(
        problem: Arc<Problem>,
        solution: (Solution, Option<Cost>),
        environment: DefaultEnvironment,
    ) -> Self {
        let mut ctx = create_insertion_context_from_solution(problem, solution, environment);
        ctx.restore();

        ctx
    }

    /// Gets total cost of the solution.
    ///
    /// Returns None if cost cannot be calculate as the context is in non-consistent state.
    pub fn get_total_cost(&self) -> Option<Cost> {
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

        let schedule_keys = self.problem.extras.get_schedule_keys()?;

        self.solution.routes.iter().try_fold(Cost::default(), |acc, route_ctx| {
            let actor = &route_ctx.route.actor;
            let distance = route_ctx.state.get_route_state::<Cost>(schedule_keys.total_distance);
            let duration = route_ctx.state.get_route_state::<Cost>(schedule_keys.total_duration);

            distance.zip(duration).map(|(&distance, &duration)| {
                acc + get_cost(&actor.vehicle.costs, distance, duration)
                    + get_cost(&actor.driver.costs, distance, duration)
            })
        })
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

/// A state key used to retrieve state values associated with a specific activity or with the whole route.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct StateKey(usize);

/// A state value which can be anything.
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
    pub state: HashMap<StateKey, StateValue>,
}

impl SolutionContext {
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

impl From<InsertionContext> for Solution {
    fn from(insertion_ctx: InsertionContext) -> Self {
        (insertion_ctx, None).into()
    }
}

impl From<(InsertionContext, Option<TelemetryMetrics>)> for Solution {
    fn from(value: (InsertionContext, Option<TelemetryMetrics>)) -> Self {
        let (insertion_ctx, telemetry) = value;
        let cost = insertion_ctx.get_total_cost().unwrap_or_default();
        let solution_ctx = insertion_ctx.solution;

        Solution {
            cost,
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
#[derive(Clone)]
pub struct RouteState {
    route_states: HashMap<usize, StateValue, BuildNoHashHasher<usize>>,
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
        let new_state = self.state.clone();

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
        RouteState { route_states: HashMap::with_capacity_and_hasher(4, BuildNoHashHasher::<usize>::default()) }
    }
}

impl RouteState {
    /// Gets value associated with key converted to given type.
    pub fn get_route_state<T: Send + Sync + 'static>(&self, key: StateKey) -> Option<&T> {
        self.route_states.get(&key.0).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key converted to given type.
    pub fn get_activity_state<T: Send + Sync + 'static>(&self, key: StateKey, activity_idx: usize) -> Option<&T> {
        self.route_states
            .get(&key.0)
            .and_then(|s| s.downcast_ref::<Vec<T>>())
            .and_then(|activity_states| activity_states.get(activity_idx))
    }

    /// Gets values associated with key and activities.
    pub fn get_activity_states<T: Send + Sync + 'static>(&self, key: StateKey) -> Option<&Vec<T>> {
        self.route_states.get(&key.0).and_then(|s| s.downcast_ref::<Vec<T>>())
    }

    /// Puts value associated with key.
    pub fn put_route_state<T: Send + Sync + 'static>(&mut self, key: StateKey, value: T) {
        self.route_states.insert(key.0, Arc::new(value));
    }

    /// Adds values associated with activities.
    pub fn put_activity_states<T: Send + Sync + 'static>(&mut self, key: StateKey, values: Vec<T>) {
        self.route_states.insert(key.0, Arc::new(values));
    }

    /// Clear all states.
    pub fn clear(&mut self) {
        self.route_states.clear();
    }
}

struct RouteCache {
    is_stale: bool,
}

/// Keeps track on how routes are used.
pub struct RegistryContext {
    registry: Registry,
    /// Index keeps track of actor mapping to empty route prototypes.
    index: HashMap<Arc<Actor>, Arc<RouteContext>>,
}

impl RegistryContext {
    /// Creates a new instance of `RouteRegistry`.
    pub fn new(goal: &GoalContext, registry: Registry) -> Self {
        let index = registry
            .all()
            .map(|actor| {
                let mut route_ctx = RouteContext::new(actor.clone());
                // NOTE: need to initialize empty route with states
                goal.accept_route_state(&mut route_ctx);

                (actor, Arc::new(route_ctx))
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
        self.registry.next().map(move |actor| self.index[&actor].as_ref())
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
            index: self.index.iter().map(|(actor, route_ctx)| (actor.clone(), route_ctx.clone())).collect(),
        }
    }

    /// Creates a deep sliced copy of `RegistryContext` keeping only specific actors data.
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

/// Provides the way to get state keys.
///
/// From performance implications, it is better to avoid using many keys: each key requires a slot
/// in route/activity state tracking collections.
#[derive(Debug, Default)]
pub struct StateKeyRegistry {
    next: usize,
}

impl StateKeyRegistry {
    /// Generates a next state key.
    /// Do not call this method if state key is not going to be used in any feature.
    pub fn next_key(&mut self) -> StateKey {
        self.next += 1;

        StateKey(self.next)
    }
}
