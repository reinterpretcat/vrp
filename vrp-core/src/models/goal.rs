#[cfg(test)]
#[path = "../../tests/unit/models/goal_test.rs"]
mod goal_test;

use crate::construction::enablers::*;
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use hashbrown::{HashMap, HashSet};
use rand::prelude::SliceRandom;
use rosomaxa::algorithms::nsga2::dominance_order;
use rosomaxa::population::Shuffled;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::ops::ControlFlow;
use std::slice::Iter;
use std::sync::Arc;

/// A type alias for a list of feature objectives.
type Objectives = Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>;
/// A type alias for a pair of alternative objectives (global and local).
type Alternative = (Vec<Objectives>, Vec<Objectives>);

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
    global_objectives: Vec<Objectives>,
    local_objectives: Vec<Objectives>,
    flatten_objectives: Objectives,
    alternatives: Option<(Vec<Alternative>, f64)>,
    constraints: Vec<Arc<dyn FeatureConstraint + Send + Sync>>,
    states: Vec<Arc<dyn FeatureState + Send + Sync>>,
}

/// Specifies a target of optimization on global and local objective levels.
#[derive(Clone, Debug)]
pub struct Target {
    /// A global hierarchical objective function specified by names of corresponding features.
    pub global: Vec<Vec<String>>,
    /// A local hierarchical objective function specified by names of corresponding features.
    pub local: Vec<Vec<String>>,
}

/// A `Goal` type defines the goal of optimization in terms of feature names.
#[derive(Clone, Debug)]
pub struct Goal {
    /// A target objective of optimization specified by user explicitly or implicitly.
    pub target: Target,
    /// Alternative objectives with their probability weight which help solver to have a better
    /// exploration during the search phase.
    pub alternatives: Option<(Vec<Target>, f64)>,
}

impl Goal {
    /// Creates instance of `Goal` without alternatives.
    pub fn no_alternatives<I>(global: I, local: I) -> Self
    where
        I: IntoIterator<Item = Vec<String>>,
    {
        let target = Target { global: global.into_iter().collect(), local: local.into_iter().collect() };

        Self { target, alternatives: None }
    }

    /// Creates instance of `Goal` with alternatives. The general idea behind alternatives is to
    /// allow search to explore solution space better, temporary switch a target objective function
    /// to one of alternatives.
    pub fn with_alternatives<I>(global: I, local: I, alternatives: (Vec<(I, I)>, f64)) -> Self
    where
        I: IntoIterator<Item = Vec<String>>,
    {
        let target = Target { global: global.into_iter().collect(), local: local.into_iter().collect() };

        let (alternatives, probability) = alternatives;
        let alternatives = alternatives
            .into_iter()
            .map(|(g, l)| Target { global: g.into_iter().collect(), local: l.into_iter().collect() })
            .collect();

        Self { target, alternatives: Some((alternatives, probability)) }
    }
}

impl GoalContext {
    /// Creates a new instance of `GoalContext` with features specified using information about
    /// hierarchy of objectives.
    pub fn new(features: &[Feature], goal: Goal) -> Result<Self, GenericError> {
        let ids_all = features
            .iter()
            .filter_map(|feature| feature.objective.as_ref().map(|_| feature.name.clone()))
            .collect::<Vec<_>>();

        let ids_unique = ids_all.iter().collect::<HashSet<_>>();
        if ids_unique.len() != ids_all.len() {
            return Err(format!(
                "some of the features are defined more than once, check ids list: {}",
                ids_all.join(",")
            )
            .into());
        }

        let check_target_map = |target: &Target| {
            [&target.global, &target.local].iter().all(|objectives| {
                let objective_ids_all = objectives.iter().flat_map(|objective| objective.iter()).collect::<Vec<_>>();
                let objective_ids_unique = objective_ids_all.iter().cloned().collect::<HashSet<_>>();
                objective_ids_all.len() == objective_ids_unique.len() && objective_ids_unique.is_subset(&ids_unique)
            })
        };

        if !check_target_map(&goal.target) {
            return Err("main target is invalid: it should contain unique ids of the features specified".into());
        }

        let feature_map = features
            .iter()
            .filter_map(|feature| feature.objective.as_ref().map(|objective| (feature.name.clone(), objective.clone())))
            .collect::<HashMap<_, _>>();

        let remap_objectives = |objective_map: &[Vec<String>]| -> Result<Vec<_>, GenericError> {
            objective_map.iter().try_fold(Vec::default(), |mut acc_outer, ids| {
                acc_outer.push(ids.iter().try_fold(Vec::default(), |mut acc_inner, id| {
                    if let Some(objective) = feature_map.get(id) {
                        acc_inner.push(objective.clone());
                        Ok(acc_inner)
                    } else {
                        Err(format!("cannot find objective for feature with id: {id}"))
                    }
                })?);

                Ok(acc_outer)
            })
        };

        let global_objectives = remap_objectives(&goal.target.global)?;
        let local_objectives = remap_objectives(&goal.target.local)?;

        let alternatives = if let Some((alternatives, probability)) = goal.alternatives {
            let alternatives = alternatives
                .into_iter()
                .map(|target| {
                    let global = remap_objectives(&target.global)?;
                    let local = remap_objectives(&target.local)?;
                    Ok((global, local))
                })
                .collect::<Result<Vec<_>, GenericError>>()?;

            if alternatives.is_empty() {
                None
            } else {
                Some((alternatives, probability))
            }
        } else {
            None
        };

        let states = features.iter().filter_map(|feature| feature.state.clone()).collect();
        let constraints = features.iter().filter_map(|feature| feature.constraint.clone()).collect();
        let flatten_objectives = global_objectives.iter().flat_map(|inners| inners.iter()).cloned().collect();

        Ok(Self { global_objectives, flatten_objectives, local_objectives, alternatives, constraints, states })
    }

    /// Creates a new instance of `GoalContext` with given feature constraints.
    pub fn with_constraints<Iter>(&self, constraints: Iter) -> Self
    where
        Iter: Iterator<Item = Arc<dyn FeatureConstraint + Send + Sync>>,
    {
        GoalContext { constraints: constraints.collect(), ..self.clone() }
    }

    /// Returns an iterator over internal feature constraints.
    pub fn constraints(&self) -> impl Iterator<Item = Arc<dyn FeatureConstraint + Send + Sync>> + '_ {
        self.constraints.iter().cloned()
    }
}

impl Debug for GoalContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("global", &self.global_objectives.len())
            .field("local", &self.local_objectives.len())
            .field("flatten", &self.flatten_objectives.len())
            .field("alternatives", &self.alternatives.is_some())
            .field("constraints", &self.constraints.len())
            .field("states", &self.states.len())
            .finish()
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
    pub constraint: Option<Arc<dyn FeatureConstraint + Send + Sync>>,
    /// An objective which models soft constraints.
    pub objective: Option<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>,
    /// A state change handler.
    pub state: Option<Arc<dyn FeatureState + Send + Sync>>,
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

/// Specifies a type for state key.
pub type StateKey = i32;

/// Provides a way to build feature with some checks.
#[derive(Default)]
pub struct FeatureBuilder(Feature);

impl FeatureBuilder {
    /// Combines multiple features into one.
    pub fn combine(name: &str, features: &[Feature]) -> Result<Feature, GenericError> {
        combine_features(name, features)
    }

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
    pub fn with_constraint<T: FeatureConstraint + Send + Sync + 'static>(mut self, constraint: T) -> Self {
        self.0.constraint = Some(Arc::new(constraint));
        self
    }

    /// Adds given objective.
    pub fn with_objective<T: FeatureObjective<Solution = InsertionContext> + Send + Sync + 'static>(
        mut self,
        objective: T,
    ) -> Self {
        self.0.objective = Some(Arc::new(objective));
        self
    }

    /// Adds given state.
    pub fn with_state<T: FeatureState + Send + Sync + 'static>(mut self, state: T) -> Self {
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
pub trait FeatureState {
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

    /// Returns unique constraint state keys used to store some state. If the data is only read, then
    /// it shouldn't be returned.
    /// Used to avoid state key interference.
    fn state_keys(&self) -> Iter<StateKey>;
}

/// Defines feature constraint behavior.
pub trait FeatureConstraint {
    /// Evaluates hard constraints violations.
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation>;

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them together having theoretically assignable
    /// job. Otherwise returns violation error code.
    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode>;
}

/// Defines feature objective behavior.
pub trait FeatureObjective: Objective {
    /// Estimates a cost of insertion.
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost;
}

impl MultiObjective for GoalContext {
    type Solution = InsertionContext;

    fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
        self.global_objectives
            .iter()
            .try_fold(Ordering::Equal, |_, objectives| {
                match dominance_order(a, b, objectives.iter().map(|o| o.as_ref())) {
                    Ordering::Equal => ControlFlow::Continue(Ordering::Equal),
                    order => ControlFlow::Break(order),
                }
            })
            .unwrap_value()
    }

    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.flatten_objectives.iter().map(|o| o.fitness(solution)))
    }

    fn get_order(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<Ordering, GenericError> {
        self.flatten_objectives
            .get(idx)
            .map(|o| o.total_order(a, b))
            .ok_or_else(|| format!("cannot get total_order with index: {idx}").into())
    }

    fn get_distance(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<f64, GenericError> {
        self.flatten_objectives
            .get(idx)
            .map(|o| o.distance(a, b))
            .ok_or_else(|| format!("cannot get distance with index: {idx}").into())
    }

    fn size(&self) -> usize {
        self.flatten_objectives.len()
    }
}

impl HeuristicObjective for GoalContext {}

impl Shuffled for GoalContext {
    /// Returns a new instance of `GoalContext` with shuffled objectives.
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self {
        let instance = self.clone();

        if let Some((alternatives, probability)) = &self.alternatives {
            assert!(!alternatives.is_empty());

            if random.is_hit(*probability) {
                let idx = random.uniform_int(0, alternatives.len() as i32 - 1) as usize;
                let (global_objectives, local_objectives) = alternatives[idx].clone();
                let flatten_objectives = global_objectives.iter().flat_map(|inners| inners.iter()).cloned().collect();

                return Self { global_objectives, flatten_objectives, local_objectives, ..instance };
            }
        }

        // NOTE: random shuffling is not very effective, so do it much less frequent
        const RANDOM_SHUFFLE_PROBABILITY: f64 = 0.01;

        if random.is_hit(RANDOM_SHUFFLE_PROBABILITY) {
            let mut global_objectives = self.global_objectives.clone();
            let mut flatten_objectives = self.flatten_objectives.clone();
            let mut local_objectives = self.local_objectives.clone();

            global_objectives.shuffle(&mut random.get_rng());
            flatten_objectives.shuffle(&mut random.get_rng());
            local_objectives.shuffle(&mut random.get_rng());

            Self { global_objectives, flatten_objectives, local_objectives, ..instance }
        } else {
            instance
        }
    }
}

impl GoalContext {
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
        self.local_objectives
            .iter()
            .map(|same_level_objectives| {
                // NOTE simply sum objective values on the same level
                // TODO: it would be nice to scale them according to their importance
                same_level_objectives.iter().map(|objective| objective.estimate(move_ctx)).sum::<Cost>()
            })
            .collect()
    }
}
