#[cfg(test)]
#[path = "../../tests/unit/models/goal_test.rs"]
mod goal_test;

use crate::construction::enablers::*;
use crate::construction::features::create_known_edge_feature;
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use rosomaxa::population::Alternative;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::iter::once;
use std::ops::ControlFlow;
use std::sync::Arc;

/// Defines Vehicle Routing Problem variant by global and local objectives:
/// A **global objective** defines the way two VRP solutions are compared to select better one:
/// for example, given the same number of assigned jobs, prefer fewer tours used instead of total
/// solution cost.
///
/// A **local objective** defines how a single VRP solution is created/modified. It specifies hard
/// constraints such as vehicle capacity, time windows, skills, etc. Also, it defines soft constraints
/// which are used to guide search in preferred by global objective direction: reduce the number of tours
/// served, maximize the total value of assigned jobs, etc.
///
/// Both, global and local objectives, are specified by individual **features**. In general, a **Feature**
/// encapsulates a single VRP aspect, such as capacity constraint for job's demand, time limitations
/// for vehicles/jobs, etc.
#[derive(Clone)]
pub struct GoalContext {
    goal: Goal,
    alternative_goals: Vec<Goal>,
    constraints: Vec<Arc<dyn FeatureConstraint>>,
    states: Vec<Arc<dyn FeatureState>>,
}

impl GoalContext {
    /// Creates a new instance of `GoalContext` with given feature constraints.
    pub fn with_constraints<Iter>(&self, constraints: Iter) -> Self
    where
        Iter: Iterator<Item = Arc<dyn FeatureConstraint>>,
    {
        GoalContext { constraints: constraints.collect(), ..self.clone() }
    }

    /// Returns an iterator over internal feature constraints.
    pub fn constraints(&self) -> impl Iterator<Item = Arc<dyn FeatureConstraint>> + '_ {
        self.constraints.iter().cloned()
    }
}

impl Debug for GoalContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("goal layers", &self.goal.layers.len())
            .field("alternatives", &self.alternative_goals.len())
            .field("constraints", &self.constraints.len())
            .field("states", &self.states.len())
            .finish()
    }
}

/// Provides a customizable way to build goal context.
pub struct GoalContextBuilder {
    main_goal: Option<Goal>,
    alternative_goals: Vec<Goal>,
    features: Vec<Feature>,
}

impl GoalContextBuilder {
    /// Creates a `GoalContextBuilder` with the given list of features.
    pub fn with_features(features: &[Feature]) -> GenericResult<Self> {
        let features = features.to_vec();
        let ids_all = features.iter().map(|feature| feature.name.as_str()).collect::<Vec<_>>();
        let ids_unique = ids_all.iter().collect::<HashSet<_>>();

        if ids_unique.len() != ids_all.len() {
            return Err(format!(
                "some of the features are defined more than once, check ids list: {}",
                ids_all.join(",")
            )
            .into());
        }

        let goal = Goal::simple(&features)?;
        let alternative_goals = vec![Self::get_heuristic_goal(&features)?];

        Ok(Self { main_goal: Some(goal), alternative_goals, features })
    }

    /// Sets a main goal of optimization.
    pub fn set_main_goal(mut self, goal: Goal) -> Self {
        self.main_goal = Some(goal);
        self
    }

    /// Sets an alternative goal of optimization.
    pub fn add_alternative_goal(mut self, goal: Goal) -> Self {
        self.alternative_goals.push(goal);
        self
    }

    /// Builds goal context.
    pub fn build(self) -> GenericResult<GoalContext> {
        let goal = self.main_goal.ok_or_else(|| GenericError::from("missing goal of optimization"))?;
        let alternative_goals = self.alternative_goals;
        let states = self.features.iter().filter_map(|feature| feature.state.clone()).collect();
        let constraints = self.features.iter().filter_map(|feature| feature.constraint.clone()).collect();

        Ok(GoalContext { goal, alternative_goals, constraints, states })
    }

    fn get_heuristic_goal(features: &[Feature]) -> GenericResult<Goal> {
        const KNOWN_EDGE_FEATURE_NAME: &str = "known_edge";
        const KEEP_SOLUTION_FITNESS: bool = false;

        let mut objective_names =
            features.iter().filter(|f| f.objective.is_some()).map(|f| f.name.as_str()).collect::<Vec<_>>();

        if objective_names.is_empty() {
            return Err(GenericError::from("no objectives specified in the goal"));
        }
        objective_names.insert(1, KNOWN_EDGE_FEATURE_NAME);

        // assuming that noone will call the feature like that...
        let known_edge = create_known_edge_feature(KNOWN_EDGE_FEATURE_NAME, KEEP_SOLUTION_FITNESS)?;
        let features = features.iter().cloned().chain(once(known_edge)).collect::<Vec<_>>();

        Goal::subset_of(&features, &objective_names)
    }
}

type TotalOrderFn =
    Arc<dyn Fn(&[Arc<dyn FeatureObjective>], &InsertionContext, &InsertionContext) -> Ordering + Send + Sync>;
type CostEstimateFn = Arc<dyn Fn(&[Arc<dyn FeatureObjective>], &MoveContext<'_>) -> Cost + Send + Sync>;
type ObjectiveLayer = (TotalOrderFn, CostEstimateFn, Vec<Arc<dyn FeatureObjective>>);

/// Specifies a goal of optimization as a list of `Feature` objectives with rules in lexicographical order.
#[derive(Clone)]
pub struct Goal {
    layers: Vec<ObjectiveLayer>,
}

impl Goal {
    /// Creates a goal using objectives from given list of features in lexicographical order.
    /// See [GoalBuilder] for more complex options.
    pub fn simple(features: &[Feature]) -> GenericResult<Self> {
        let mut builder = GoalBuilder::default();
        let names = features.iter().filter(|f| f.objective.is_some()).map(|f| &f.name);

        for name in names {
            builder = Self::add_with_name(builder, features, name)?;
        }

        builder.build()
    }

    /// Creates a goal using feature names from the given list. Objectives are defined in lexicographical order.
    pub fn subset_of<S: AsRef<str>>(features: &[Feature], names: &[S]) -> GenericResult<Self> {
        let mut builder = GoalBuilder::default();

        for name in names {
            builder = Self::add_with_name(builder, features, name.as_ref())?;
        }

        builder.build()
    }

    fn add_with_name(builder: GoalBuilder, features: &[Feature], name: &str) -> GenericResult<GoalBuilder> {
        let feature = features
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| GenericError::from(format!("cannot find a feature with given name: '{name}'")))?;

        let objective = feature
            .objective
            .clone()
            .ok_or_else(|| GenericError::from(format!("feature '{name}' has no objective")))?;

        Ok(builder.add_single(objective))
    }
}

impl Goal {
    /// Compares two solutions from optimization goal point of view and returns their comparison order.
    pub fn total_order(&self, a: &InsertionContext, b: &InsertionContext) -> Ordering {
        self.layers
            .iter()
            .try_fold(Ordering::Equal, |_, (total_order_fn, _, objectives)| {
                match (total_order_fn)(objectives.as_slice(), a, b) {
                    Ordering::Equal => ControlFlow::Continue(Ordering::Equal),
                    order => ControlFlow::Break(order),
                }
            })
            .unwrap_value()
    }

    /// Estimates insertion cost (penalty) of the refinement move.
    pub fn estimate(&self, move_ctx: &MoveContext<'_>) -> InsertionCost {
        self.layers.iter().map(|(_, estimate_fn, objectives)| (estimate_fn)(objectives.as_slice(), move_ctx)).collect()
    }

    /// Calculates solution's fitness.
    pub fn fitness<'a>(&'a self, solution: &'a InsertionContext) -> impl Iterator<Item = Float> + 'a {
        self.layers.iter().flat_map(|(_, _, objectives)| objectives.iter()).map(|objective| objective.fitness(solution))
    }
}

/// Builds a [Goal] - a goal of optimization - composing multiple layers from objective functions
/// in lexicographical order.
#[derive(Default)]
pub struct GoalBuilder {
    layers: Vec<ObjectiveLayer>,
}

impl GoalBuilder {
    /// Add a layer which consists of one objective function with a given feature name.
    pub fn add_single(mut self, objective: Arc<dyn FeatureObjective>) -> Self {
        // NOTE: indices are controlled internally
        self.layers.push((
            Arc::new(|objectives, a, b| {
                let fitness_a = objectives[0].fitness(a);
                let fitness_b = objectives[0].fitness(b);

                // NOTE total_cmp distinguishes between positive zero and negative zero while
                // logically they are the same in this context
                if fitness_a == 0. && fitness_b == 0. { Ordering::Equal } else { fitness_a.total_cmp(&fitness_b) }
            }),
            Arc::new(|objectives, move_ctx| objectives[0].estimate(move_ctx)),
            vec![objective],
        ));
        self
    }

    /// Add a layer which consists of one or many objective function with a given feature name and
    /// a custom `GoalResolver`.
    pub fn add_multi<TO, CE>(
        mut self,
        objectives: &[Arc<dyn FeatureObjective>],
        total_order_fn: TO,
        cost_estimate_fn: CE,
    ) -> Self
    where
        TO: Fn(&[Arc<dyn FeatureObjective>], &InsertionContext, &InsertionContext) -> Ordering + Send + Sync + 'static,
        CE: Fn(&[Arc<dyn FeatureObjective>], &MoveContext<'_>) -> Cost + Send + Sync + 'static,
    {
        self.layers.push((Arc::new(total_order_fn), Arc::new(cost_estimate_fn), objectives.to_vec()));
        self
    }

    /// Builds a [Goal] of optimization using features provided.
    pub fn build(self) -> GenericResult<Goal> {
        if self.layers.is_empty() {
            return Err(GenericError::from("no objectives specified in the goal"));
        }

        Ok(Goal { layers: self.layers })
    }
}

/// An individual feature which is used to build a specific VRP variant, e.g., capacity restriction,
/// job values, etc. Each feature consists of three optional parts (but at least one should be defined):
///
/// * **constraint**: an invariant which should be hold to have a feasible VRP solution in the end.
///   A good examples are hard constraints such as capacity, time, travel limits, etc.
///
/// * **objective**: an objective of the optimization such as minimization of unassigned jobs or tours.
///   All objectives form together a hierarchy which describes a goal of optimization, including
///   various soft constraints: assignment of preferred jobs, optional breaks, etc. This helps to
///   guide the search on the global objective level (e.g. comparison of various solutions in order to
///   find out which one is "better") and local objective level (e.g. which job should be inserted next
///   into specific solution).
///
/// * **state**: the corresponding cached data of constraint/objective to speed up/control their evaluations.
///
/// As mentioned above, at least one part should be defined. Some rules of thumb:
/// * each soft constraint requires an objective so that the goal of optimization is reflected on global
///   and local levels
/// * hard constraint can be defined without objective as this is an invariant
/// * state should be used to avoid expensive calculations during insertion evaluation phase.
///   `FeatureObjective::estimate` and `FeatureConstraint::evaluate` methods are called during this phase.
///   Additionally, it can be used to do some solution modifications at `FeatureState::accept_solution_state`.
#[derive(Clone, Default)]
pub struct Feature {
    /// An unique id of the feature.
    pub name: String,
    /// A hard constraint.
    pub constraint: Option<Arc<dyn FeatureConstraint>>,
    /// An objective which models soft constraints.
    pub objective: Option<Arc<dyn FeatureObjective>>,
    /// A state change handler.
    pub state: Option<Arc<dyn FeatureState>>,
}

/// Specifies a result of hard route constraint check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintViolation {
    /// Violation code which is used as marker of specific constraint violated.
    pub code: ViolationCode,
    /// True if further insertions should not be attempted.
    pub stopped: bool,
}

impl ConstraintViolation {
    /// A constraint violation failure with stopped set to true.
    pub fn fail(code: ViolationCode) -> Option<Self> {
        Some(ConstraintViolation { code, stopped: true })
    }

    /// A constraint violation failure with stopped set to false.
    pub fn skip(code: ViolationCode) -> Option<Self> {
        Some(ConstraintViolation { code, stopped: false })
    }

    /// No constraint violation.
    pub fn success() -> Option<Self> {
        None
    }
}

/// Specifies a type for constraint violation code.
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct ViolationCode(pub i32);

impl ViolationCode {
    /// Returns an unknown violation code.
    pub fn unknown() -> Self {
        Self(-1)
    }

    /// Checks whether violation code is unknown.
    pub fn is_unknown(&self) -> bool {
        self.0 == -1
    }
}

impl From<i32> for ViolationCode {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl Display for ViolationCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

/// Provides a way to build feature with some checks.
#[derive(Default)]
pub struct FeatureBuilder(Feature);

impl FeatureBuilder {
    /// Creates a builder from another feature.
    pub fn from_feature(feature: Feature) -> Self {
        Self(feature)
    }

    /// Sets given name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.0.name = name.to_string();
        self
    }

    /// Adds given constraint.
    pub fn with_constraint<T: FeatureConstraint + 'static>(mut self, constraint: T) -> Self {
        self.0.constraint = Some(Arc::new(constraint));
        self
    }

    /// Adds given objective.
    pub fn with_objective<T: FeatureObjective + 'static>(mut self, objective: T) -> Self {
        self.0.objective = Some(Arc::new(objective));
        self
    }

    /// Adds given state.
    pub fn with_state<T: FeatureState + 'static>(mut self, state: T) -> Self {
        self.0.state = Some(Arc::new(state));
        self
    }

    /// Tries to build a feature.
    pub fn build(self) -> Result<Feature, GenericError> {
        let feature = self.0;

        if feature.name == String::default() {
            return Err("features with default id are not allowed".into());
        }

        if feature.constraint.is_none() && feature.objective.is_none() && feature.state.is_none() {
            Err("empty feature is not allowed".into())
        } else {
            Ok(feature)
        }
    }
}

/// Provides the way to modify solution state when the search is performed.
pub trait FeatureState: Send + Sync {
    /// Notifies a state that given routes (indices) and jobs cannot be assigned due to constraint violations.
    /// This method can be used to modify solution context to help resolve some limitations imposed by
    /// constraints and, generally, can modify solution context.
    /// If some action was taken which might help to assign given jobs to given routes, then true
    /// should be returned. **Please note**, if this method wrongly returns true, it might cause infinite
    /// loops in an insertion evaluation process.
    /// The default implementation returns false, which is safe and ok for most of the features.
    fn notify_failure(&self, _solution_ctx: &mut SolutionContext, _route_indices: &[usize], _jobs: &[Job]) -> bool {
        false
    }

    /// Accept insertion of a specific job into the route.
    /// Called once a job has been inserted into a solution represented via `solution_ctx`.
    /// Target route is defined by `route_index` which refers to `routes` collection in solution context.
    /// Inserted job is `job`.
    /// This method can call `accept_route_state` internally.
    /// This method should NOT modify the number of job activities in the tour.
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job);

    /// Accept route and updates its state to allow more efficient constraint checks.
    /// This method should NOT modify the number of job activities in the tour.
    fn accept_route_state(&self, route_ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// This method called twice: before insertion of all jobs starts and when it ends.
    /// Please note that it is important to update only stale routes as this allows avoiding
    /// update of non-changed route states.
    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext);
}

/// Defines feature constraint behavior.
pub trait FeatureConstraint: Send + Sync {
    /// Evaluates hard constraints violations.
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation>;

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them having a theoretically assignable
    /// job. Otherwise, returns violation error code.
    /// Default implementation returns an error with default [ViolationCode].
    fn merge(&self, _source: Job, _candidate: Job) -> Result<Job, ViolationCode> {
        Err(ViolationCode::default())
    }
}

/// Defines feature's objective function behavior.
pub trait FeatureObjective: Send + Sync {
    /// An objective fitness value for the given `solution`.
    fn fitness(&self, solution: &InsertionContext) -> Cost;

    /// Estimates the cost of insertion.
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost;
}

impl HeuristicObjective for GoalContext {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        self.goal.total_order(a, b)
    }
}

impl Alternative for GoalContext {
    fn maybe_new(&self, random: &dyn Random) -> Self {
        // TODO pass heuristic statistic here to vary probability based on convergence.
        const RANDOM_ALTERNATIVE_PROBABILITY: Float = 0.1;

        if !self.alternative_goals.is_empty() && random.is_hit(RANDOM_ALTERNATIVE_PROBABILITY) {
            let idx = random.uniform_int(0, self.alternative_goals.len() as i32 - 1) as usize;
            self.get_alternative(idx)
        } else {
            self.clone()
        }
    }
}

impl GoalContext {
    fn get_alternative(&self, idx: usize) -> Self {
        let goal = self.alternative_goals[idx].clone();

        Self { goal, ..self.clone() }
    }

    /// Returns goals with alternative objectives.
    pub(crate) fn get_alternatives(&self) -> impl Iterator<Item = Self> + '_ {
        self.alternative_goals.iter().enumerate().map(|(idx, _)| self.get_alternative(idx))
    }

    /// Accepts job insertion.
    pub fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        accept_insertion_with_states(&self.states, solution_ctx, route_index, job)
    }

    /// Accepts route state.
    pub fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        accept_route_state_with_states(&self.states, route_ctx)
    }

    /// Accepts solution state.
    pub fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        accept_solution_state_with_states(&self.states, solution_ctx);
    }

    /// Notifies about a failed attempt to insert given jobs into given routes (indices).
    /// Returns true if failure is some attempt to handle failure was performed and retry can be
    /// performed.
    pub fn notify_failure(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool {
        notify_failure_with_states(&self.states, solution_ctx, route_indices, jobs)
    }

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them having a theoretically assignable
    /// job. Otherwise returns violation error code.
    pub fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        merge_with_constraints(&self.constraints, source, candidate)
    }

    /// Evaluates feasibility of the refinement move.
    pub fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        evaluate_with_constraints(&self.constraints, move_ctx)
    }

    /// Estimates insertion cost (penalty) of the refinement move.
    pub fn estimate(&self, move_ctx: &MoveContext<'_>) -> InsertionCost {
        self.goal.estimate(move_ctx)
    }

    /// Calculates solution's fitness.
    pub fn fitness<'a>(&'a self, solution: &'a InsertionContext) -> impl Iterator<Item = Float> + 'a {
        self.goal.fitness(solution)
    }
}

impl Debug for Feature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("name", &self.name)
            .field("constraint", &self.constraint.is_some())
            .field("objective", &self.objective.is_some())
            .field("state", &self.state.is_some())
            .finish()
    }
}
