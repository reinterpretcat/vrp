#[cfg(test)]
#[path = "../../tests/unit/models/goal_test.rs"]
mod goal_test;

use crate::construction::enablers::*;
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use rand::prelude::SliceRandom;
use rosomaxa::evolution::objectives::dominance_order;
use rosomaxa::population::Shuffled;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::ControlFlow;
use std::sync::Arc;

/// A type alias for a list of feature objectives.
type Objectives = Vec<Arc<dyn FeatureObjective>>;

/// Defines Vehicle Routing Problem variant by global and local objectives:
/// A **global objective** defines the way two VRP solutions are compared in order to select better one:
/// for example, given the same amount of assigned jobs, prefer less tours used instead of total
/// solution cost.
///
/// A **local objective** defines how single VRP solution is created/modified. It specifies hard
/// constraints such as vehicle capacity, time windows, skills, etc. Also it defines soft constraints
/// which are used to guide search in preferred by global objective direction: reduce amount of tours
/// served, maximize total value of assigned jobs, etc.
///
/// Both, global and local objectives, are specified by individual **features**. In general, a **Feature**
/// encapsulates a single VRP aspect, such as capacity constraint for job' demand, time limitations
/// for vehicles/jobs, etc.
#[derive(Clone, Default)]
pub struct GoalContext {
    global_objectives: Objectives,
    local_objectives: Objectives,
    alternatives: Vec<(Objectives, Objectives, f64)>,
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
            .field("global", &self.global_objectives.len())
            .field("local", &self.local_objectives.len())
            .field("alternatives", &self.alternatives.len())
            .field("constraints", &self.constraints.len())
            .field("states", &self.states.len())
            .finish()
    }
}

/// Provides a customizable way to build goal context.
pub struct GoalContextBuilder {
    main_goal: Option<(Vec<Feature>, Vec<Feature>)>,
    alternative_goals: Vec<(Vec<Feature>, Vec<Feature>, f64)>,
    features: Vec<Feature>,
}

impl GoalContextBuilder {
    /// Creates a `GoalBuilder` with the given list of features.
    pub fn with_features(features: Vec<Feature>) -> GenericResult<Self> {
        let ids_all = features.iter().map(|feature| feature.name.as_str()).collect::<Vec<_>>();
        let ids_unique = ids_all.iter().collect::<HashSet<_>>();

        if ids_unique.len() != ids_all.len() {
            return Err(format!(
                "some of the features are defined more than once, check ids list: {}",
                ids_all.join(",")
            )
            .into());
        }

        Ok(Self { main_goal: None, alternative_goals: Vec::default(), features })
    }

    /// Sets a main goal of optimization.
    pub fn set_goal(mut self, global: &[&str], local: &[&str]) -> GenericResult<Self> {
        let global = self.get_features(global)?;
        let local = self.get_features(local)?;

        self.main_goal = Some((global, local));

        Ok(self)
    }

    /// Sets an alternative goal of optimization.
    pub fn add_alternative(mut self, global: &[&str], local: &[&str], weight: f64) -> GenericResult<Self> {
        let global = self.get_features(global)?;
        let local = self.get_features(local)?;

        self.alternative_goals.push((global, local, weight));

        Ok(self)
    }

    fn get_features(&self, names: &[&str]) -> GenericResult<Vec<Feature>> {
        names
            .iter()
            .map(|n| {
                self.features
                    .iter()
                    .find(|f| f.name.eq(n))
                    .cloned()
                    .ok_or_else(|| GenericError::from(format!("unknown feature name: '{n}'")))
            })
            .collect::<Result<Vec<_>, _>>()
    }

    /// Builds goal context.
    pub fn build(self) -> GenericResult<GoalContext> {
        let feature_objective_map = self
            .features
            .iter()
            .filter_map(|feature| feature.objective.as_ref().map(|objective| (feature.name.clone(), objective.clone())))
            .collect::<HashMap<_, _>>();

        let remap_objectives = |features: &[Feature]| -> GenericResult<Vec<_>> {
            features
                .iter()
                .map(|f| {
                    if let Some(objective) = feature_objective_map.get(&f.name) {
                        Ok(objective.clone())
                    } else {
                        Err(format!("cannot find objective for feature with name: {}", f.name).into())
                    }
                })
                .collect::<Result<_, GenericError>>()
        };

        let (global_objectives, local_objectives) = if let Some((global, local)) = self.main_goal {
            (remap_objectives(&global)?, remap_objectives(&local)?)
        } else {
            return Err("missing main goal of optimization".into());
        };

        let alternatives = self
            .alternative_goals
            .into_iter()
            .map(|(global, local, weight)| Ok((remap_objectives(&global)?, remap_objectives(&local)?, weight)))
            .collect::<GenericResult<Vec<_>>>()?;

        let states = self.features.iter().filter_map(|feature| feature.state.clone()).collect();
        let constraints = self.features.iter().filter_map(|feature| feature.constraint.clone()).collect();

        Ok(GoalContext { global_objectives, local_objectives, alternatives, constraints, states })
    }
}

/// An individual feature which is used to build a specific VRP variant, e.g. capacity restriction,
/// job values, etc. Each feature consists of three optional parts (but at least one should be defined):
///
/// * **constraint**: an invariant which should be hold to have a feasible VRP solution in the end.
/// A good examples are hard constraints such as capacity, time, travel limits, etc.
///
/// * **objective**: an objective of the optimization such as minimization of unassigned jobs or tours.
///  All objectives form together a hierarchy which describes a goal of optimization, including
///  various soft constraints: assignment of preferred jobs, optional breaks, etc. This helps to
///  guide the search on the global objective level (e.g. comparison of various solutions in order to
///  find out which one is "better") and local objective level (e.g. which job should be inserted next
///  into specific solution).
///
/// * **state**: the corresponding cached data of constraint/objective to speedup/control their evaluations.
///
/// As mentioned above, at least one part should be defined. Some rules of thumb:
/// * each soft constraint requires an objective so that goal of optimization is reflected on global
///   and local levels
/// * hard constraint can be defined without objective as this is an invariant
/// * state should be used to avoid expensive calculations during insertion evaluation phase.
///   `FeatureObjective::estimate` and `FeatureConstraint::evaluate` methods are called during this phase.
///  Additionally, it can be used to do some solution modifications at `FeatureState::accept_solution_state`.
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

/// Specifies result of hard route constraint check.
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
pub type ViolationCode = i32;

/// Provides a way to build feature with some checks.
#[derive(Default)]
pub struct FeatureBuilder(Feature);

impl FeatureBuilder {
    /// Creates a builder from another feature
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

    /// Tries to builds a feature.
    pub fn build(self) -> Result<Feature, GenericError> {
        let feature = self.0;

        if feature.name == String::default() {
            return Err("features with default id are not allowed".into());
        }

        if feature.constraint.is_none() && feature.objective.is_none() {
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
    /// loops in insertion evaluation process.
    /// Default implementation returns false which is safe and ok for most of the features.
    fn notify_failure(&self, _solution_ctx: &mut SolutionContext, _route_indices: &[usize], _jobs: &[Job]) -> bool {
        false
    }

    /// Accept insertion of specific job into the route.
    /// Called once job has been inserted into solution represented via `solution_ctx`.
    /// Target route is defined by `route_index` which refers to `routes` collection in solution context.
    /// Inserted job is `job`.
    /// This method can call `accept_route_state` internally.
    /// This method should NOT modify amount of job activities in the tour.
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job);

    /// Accept route and updates its state to allow more efficient constraint checks.
    /// This method should NOT modify amount of job activities in the tour.
    fn accept_route_state(&self, route_ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// This method called twice: before insertion of all jobs starts and when it ends.
    /// Please note, that it is important to update only stale routes as this allows to avoid
    /// updating non changed route states.
    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext);
}

/// Defines feature constraint behavior.
pub trait FeatureConstraint: Send + Sync {
    /// Evaluates hard constraints violations.
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation>;

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them together having theoretically assignable
    /// job. Otherwise returns violation error code.
    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode>;
}

/// Defines feature objective behavior.
pub trait FeatureObjective: Send + Sync {
    /// An objective defines a total ordering between any two solution values.
    fn total_order(&self, a: &InsertionContext, b: &InsertionContext) -> Ordering {
        compare_floats(self.fitness(a), self.fitness(b))
    }

    /// An objective fitness values for given `solution`.
    fn fitness(&self, solution: &InsertionContext) -> f64;

    /// Estimates a cost of insertion.
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost;
}

impl HeuristicObjective for GoalContext {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        self.global_objectives
            .iter()
            .try_fold(Ordering::Equal, |_, objective| {
                match dominance_order(a, b, std::iter::once(|a, b| objective.total_order(a, b))) {
                    Ordering::Equal => ControlFlow::Continue(Ordering::Equal),
                    order => ControlFlow::Break(order),
                }
            })
            .unwrap_value()
    }

    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.global_objectives.iter().map(|o| o.fitness(solution)))
    }
}

impl Shuffled for GoalContext {
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self {
        const RANDOM_ALTERNATIVE_PROBABILITY: f64 = 0.05;
        const RANDOM_SHUFFLE_PROBABILITY: f64 = 0.001;

        if !self.alternatives.is_empty() && random.is_hit(RANDOM_ALTERNATIVE_PROBABILITY) {
            let idx = random.uniform_int(0, self.alternatives.len() as i32 - 1) as usize;
            return self.get_alternative(idx);
        }

        if random.is_hit(RANDOM_SHUFFLE_PROBABILITY) {
            self.get_shuffled(random)
        } else {
            self.clone()
        }
    }
}

impl GoalContext {
    fn get_alternative(&self, idx: usize) -> Self {
        let (global_objectives, local_objectives, _) = self.alternatives[idx].clone();

        Self { global_objectives, local_objectives, ..self.clone() }
    }

    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self {
        let instance = self.clone();

        let mut global_objectives = self.global_objectives.clone();
        let mut local_objectives = self.local_objectives.clone();

        global_objectives.shuffle(&mut random.get_rng());
        local_objectives.shuffle(&mut random.get_rng());

        Self { global_objectives, local_objectives, ..instance }
    }

    /// Returns goals with alternative objectives.
    pub(crate) fn get_alternatives(&self) -> impl Iterator<Item = Self> + '_ {
        self.alternatives.iter().enumerate().map(|(idx, _)| self.get_alternative(idx))
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

    /// Notifies about failed attempt to insert given jobs into given routes (indices).
    /// Returns true if failure is some attempt to handle failure was performed and retry can be
    /// performed.
    pub fn notify_failure(&self, solution_ctx: &mut SolutionContext, route_indices: &[usize], jobs: &[Job]) -> bool {
        notify_failure_with_states(&self.states, solution_ctx, route_indices, jobs)
    }

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them together having theoretically assignable
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
        self.local_objectives.iter().map(|objective| objective.estimate(move_ctx)).collect()
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
