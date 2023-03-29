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
use std::slice::Iter;
use std::sync::Arc;

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
    pub(crate) hierarchical_objectives: Vec<Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>>,
    pub(crate) flat_objectives: Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>,
    pub(crate) local_objectives: Vec<Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>>,
    pub(crate) constraints: Vec<Arc<dyn FeatureConstraint + Send + Sync>>,
    pub(crate) states: Vec<Arc<dyn FeatureState + Send + Sync>>,
}

impl GoalContext {
    /// Creates a new instance of `VrpVariant` with features specified using information about
    /// hierarchy of objectives.
    pub fn new(
        features: &[Feature],
        global_objective_map: &[Vec<String>],
        local_objective_map: &[Vec<String>],
    ) -> Result<Self, String> {
        let ids_all = features
            .iter()
            .filter_map(|feature| feature.objective.as_ref().map(|_| feature.name.clone()))
            .collect::<Vec<_>>();

        let ids_unique = ids_all.iter().collect::<HashSet<_>>();
        if ids_unique.len() != ids_all.len() {
            return Err(format!(
                "some of the features are defined more than once, check ids list: {}",
                ids_all.join(",")
            ));
        }

        let check_objective_map = |objective_map: &[Vec<String>]| {
            let objective_ids_all = objective_map.iter().flat_map(|objective| objective.iter()).collect::<Vec<_>>();
            let objective_ids_unique = objective_ids_all.iter().cloned().collect::<HashSet<_>>();
            objective_ids_all.len() == objective_ids_unique.len() && objective_ids_unique.is_subset(&ids_unique)
        };

        if !check_objective_map(global_objective_map) {
            return Err(
                "global objective map is invalid: it should contain unique ids of the features specified".to_string()
            );
        }

        if !check_objective_map(local_objective_map) {
            return Err(
                "local objective map is invalid: it should contain unique ids of the features specified".to_string()
            );
        }

        let feature_map = features
            .iter()
            .filter_map(|feature| feature.objective.as_ref().map(|objective| (feature.name.clone(), objective.clone())))
            .collect::<HashMap<_, _>>();

        let remap_objectives = |objective_map: &[Vec<String>]| -> Result<Vec<_>, String> {
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

        let hierarchical_objectives = remap_objectives(global_objective_map)?;
        let local_objectives = remap_objectives(local_objective_map)?;

        let states = features.iter().filter_map(|feature| feature.state.clone()).collect();
        let constraints = features.iter().filter_map(|feature| feature.constraint.clone()).collect();
        let flat_objectives = hierarchical_objectives.iter().flat_map(|inners| inners.iter()).cloned().collect();

        Ok(Self { hierarchical_objectives, flat_objectives, local_objectives, constraints, states })
    }
}

/// An individual feature which is used to build a specific VRP variant, e.g. capacity restriction,
/// job values, etc. Each feature consists of three optional parts (but at least one should be defined):
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
/// * **state**: the corresponding cached data of constraint/objective to speedup their evaluations.
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
pub struct FeatureBuilder {
    feature: Feature,
}

impl FeatureBuilder {
    /// Combines multiple features into one.
    pub fn combine(name: &str, features: &[Feature]) -> Result<Feature, String> {
        combine_features(name, features)
    }

    /// Creates a builder from another feature
    pub fn from_feature(other: Feature) -> Self {
        Self { feature: other }
    }

    /// Sets given name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.feature.name = name.to_string();
        self
    }

    /// Adds given constraint.
    pub fn with_constraint<T: FeatureConstraint + Send + Sync + 'static>(mut self, constraint: T) -> Self {
        self.feature.constraint = Some(Arc::new(constraint));
        self
    }

    /// Adds given objective.
    pub fn with_objective<T: FeatureObjective<Solution = InsertionContext> + Send + Sync + 'static>(
        mut self,
        objective: T,
    ) -> Self {
        self.feature.objective = Some(Arc::new(objective));
        self
    }

    /// Adds given state.
    pub fn with_state<T: FeatureState + Send + Sync + 'static>(mut self, state: T) -> Self {
        self.feature.state = Some(Arc::new(state));
        self
    }

    /// Tries to builds a feature.
    pub fn build(self) -> Result<Feature, String> {
        let feature = self.feature;

        if feature.name == String::default() {
            return Err("features with default id are not allowed".to_string());
        }

        if feature.constraint.is_none() && feature.objective.is_none() {
            Err("empty feature is not allowed".to_string())
        } else {
            Ok(feature)
        }
    }
}

/// Controls a cached state of the given feature.
pub trait FeatureState {
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
        unwrap_from_result(self.hierarchical_objectives.iter().try_fold(Ordering::Equal, |_, objectives| {
            match dominance_order(a, b, objectives.iter().map(|o| o.as_ref())) {
                Ordering::Equal => Ok(Ordering::Equal),
                order => Err(order),
            }
        }))
    }

    fn fitness<'a>(&'a self, solution: &'a Self::Solution) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.flat_objectives.iter().map(|o| o.fitness(solution)))
    }

    fn get_order(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<Ordering, String> {
        self.flat_objectives
            .get(idx)
            .map(|o| o.total_order(a, b))
            .ok_or_else(|| format!("cannot get total_order with index: {idx}"))
    }

    fn get_distance(&self, a: &Self::Solution, b: &Self::Solution, idx: usize) -> Result<f64, String> {
        self.flat_objectives
            .get(idx)
            .map(|o| o.distance(a, b))
            .ok_or_else(|| format!("cannot get distance with index: {idx}"))
    }

    fn size(&self) -> usize {
        self.flat_objectives.len()
    }
}

impl HeuristicObjective for GoalContext {}

impl Shuffled for GoalContext {
    /// Returns a new instance of `ObjectiveCost` with shuffled objectives.
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self {
        let mut hierarchical_objectives = self.hierarchical_objectives.clone();

        hierarchical_objectives.shuffle(&mut random.get_rng());

        Self { hierarchical_objectives, ..self.clone() }
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
        self.local_objectives.iter().fold(InsertionCost::default(), |acc, objectives| {
            objectives
                .iter()
                .map(|objective| objective.estimate(move_ctx))
                .zip(acc.into_iter().chain(std::iter::repeat(Cost::default())))
                .map(|(a, b)| {
                    // NOTE we simply add cost values of two
                    a + b
                })
                .collect()
        })
    }
}
