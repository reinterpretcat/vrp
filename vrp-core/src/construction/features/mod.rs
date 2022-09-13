//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use hashbrown::HashSet;
use rosomaxa::prelude::*;
use std::iter::repeat;
use std::slice::Iter;
use std::sync::Arc;

pub mod capacity;
pub mod fleet_usage;
pub mod locked_jobs;
pub mod minimize_unassigned;
pub mod shared_resource;
pub mod total_value;
pub mod tour_limits;
pub mod tour_order;
pub mod transport;
pub mod work_balance;

// TODO move state keys here

/// Keys for balancing objectives.
const BALANCE_MAX_LOAD_KEY: i32 = 20;
const BALANCE_ACTIVITY_KEY: i32 = 21;
const BALANCE_DISTANCE_KEY: i32 = 22;
const BALANCE_DURATION_KEY: i32 = 23;

/// An individual feature which is used to build a specific VRP aspect (variant), e.g. capacity restriction,
/// job priority, etc. Each feature consists of three optional parts (but at least one should be defined):
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
#[derive(Clone, Default)]
pub struct Feature {
    constraint: Option<Arc<dyn FeatureConstraint + Send + Sync>>,
    objective: Option<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>,
    state: Option<Arc<dyn FeatureState + Send + Sync>>,
}

/// Specifies result of hard route constraint check.
#[derive(Clone, Debug)]
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

/// A hierarchical cost of job's insertion.
pub type InsertionCost = tinyvec::TinyVec<[Cost; 8]>;

/// Provides a way to build feature with some checks.
#[derive(Default)]
pub struct FeatureBuilder {
    feature: Feature,
}

impl FeatureBuilder {
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

/// Provides a way to maintain multiple features according their hierarchical priorities.
pub struct FeatureRegistry {
    constraints: Vec<Arc<dyn FeatureConstraint + Send + Sync>>,
    objectives: Vec<Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>>,
    states: Vec<Arc<dyn FeatureState + Send + Sync>>,
}

impl FeatureRegistry {
    /// Creates a new instance of `FeatureRegistry` from hierarchy of the features.
    /// Hierarchy of the features could be the same as the desired global objective hierarchy or
    /// include some extra objectives for soft constraints.
    pub fn new(prioritized_features: &[Vec<Feature>]) -> Result<Self, String> {
        let objectives = prioritized_features
            .iter()
            .map(|features| features.iter().filter_map(|feature| feature.objective.clone()).collect())
            .collect();

        let features: Vec<Feature> = prioritized_features.iter().flatten().cloned().collect();

        features.iter().filter_map(|feature| feature.state.as_ref()).flat_map(|state| state.state_keys()).try_fold(
            HashSet::<StateKey>::default(),
            |mut acc, key| {
                if !acc.insert(*key) {
                    Err(format!("attempt to register constraint with key duplication: {}", key))
                } else {
                    Ok(acc)
                }
            },
        )?;

        let states = features.iter().filter_map(|feature| feature.state.clone()).collect();
        let constraints = features.into_iter().filter_map(|feature| feature.constraint).collect();

        Ok(Self { states, constraints, objectives })
    }

    /// Accepts job insertion.
    pub fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        let activities = solution_ctx.routes.get_mut(route_index).unwrap().route.tour.job_activity_count();
        self.states.iter().for_each(|state| state.accept_insertion(solution_ctx, route_index, job));
        assert_eq!(activities, solution_ctx.routes.get_mut(route_index).unwrap().route.tour.job_activity_count());
    }

    /// Accepts route state.
    pub fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        if route_ctx.is_stale() {
            route_ctx.state_mut().clear();

            let activities = route_ctx.route.tour.job_activity_count();
            self.states.iter().for_each(|state| state.accept_route_state(route_ctx));
            assert_eq!(activities, route_ctx.route.tour.job_activity_count());

            route_ctx.mark_stale(false);
        }
    }

    /// Accepts solution state.
    pub fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        let has_changes = |ctx: &SolutionContext, previous_state: (usize, usize, usize)| {
            let (required, ignored, unassigned) = previous_state;
            required != ctx.required.len() || ignored != ctx.ignored.len() || unassigned != ctx.unassigned.len()
        };

        let _ = (0..).try_fold((usize::MAX, usize::MAX, usize::MAX), |(required, ignored, unassigned), counter| {
            // NOTE if any job promotion occurs, then we might need to recalculate states.
            // As it is hard to maintain dependencies between different modules, we reset process to
            // beginning. However we do not expect recalculation to happen often, so this condition
            // here is to prevent infinite loops and signalize about error in pipeline configuration
            assert_ne!(counter, 100);

            if has_changes(solution_ctx, (required, ignored, unassigned)) {
                let required = solution_ctx.required.len();
                let ignored = solution_ctx.ignored.len();
                let unassigned = solution_ctx.unassigned.len();

                self.states
                    .iter()
                    .try_for_each(|state| {
                        state.accept_solution_state(solution_ctx);
                        if has_changes(solution_ctx, (required, ignored, unassigned)) {
                            Err(())
                        } else {
                            Ok(())
                        }
                    })
                    .map(|_| (required, ignored, unassigned))
                    .or(Ok((usize::MAX, usize::MAX, usize::MAX)))
            } else {
                Err(())
            }
        });

        solution_ctx.routes.iter_mut().for_each(|route_ctx| {
            route_ctx.mark_stale(false);
        })
    }

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them together having theoretically assignable
    /// job. Otherwise returns violation error code.
    pub fn merge_constrained(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        self.constraints.iter().try_fold(source, |acc, constraint| constraint.merge(acc, candidate.clone()))
    }

    /// Evaluates feasibility of the refinement move.
    pub fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        unwrap_from_result(self.constraints.iter().try_fold(None, |_, constraint| {
            let result = constraint.evaluate(move_ctx);
            let is_stopped = result.as_ref().map_or(false, |result| result.stopped);

            if is_stopped {
                Err(result)
            } else {
                Ok(result)
            }
        }))
    }

    /// Estimates insertion cost (penalty) of the refinement move.
    pub fn estimate(&self, move_ctx: &MoveContext<'_>) -> InsertionCost {
        self.objectives.iter().fold(InsertionCost::default(), |acc, objectives| {
            objectives
                .iter()
                .map(|objective| objective.estimate(move_ctx))
                .zip(acc.into_iter().chain(repeat(Cost::default())))
                .map(|(a, b)| {
                    // TODO: merging two values will reintroduce problem with weightning coefficients
                    //     use a flat structure for insertion cost with priority map and apply total ordering?
                    a + b
                })
                .collect()
        })
    }
}
