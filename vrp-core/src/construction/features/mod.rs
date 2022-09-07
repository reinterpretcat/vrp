//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use hashbrown::HashSet;
use rosomaxa::prelude::*;
use std::iter::repeat;
use std::slice::Iter;
use std::sync::Arc;

// TODO add more descriptive documentation

/// An individual feature to build a specific VRP aspect (variant), e.g. capacity restriction, job
/// priority, etc. Each feature can consist of three optional concepts (but at least one should be defined):
/// * **constraint**: an invariant which should be hold all the time to make feasible VRP solution,
/// e.g. capacity/time constraints.
/// * **objective**: an objective of an optimization such as minimize amount of unassigned jobs or
///  prefer some specific jobs for assignment.
/// * **state**: the corresponding cached data for constraint/objective to speedup their evaluations.
#[derive(Clone)]
pub struct Feature {
    constraint: Option<Arc<dyn FeatureConstraint + Send + Sync>>,
    objective: Option<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>,
    state: Option<Arc<dyn FeatureState + Send + Sync>>,
}

pub struct FeatureBuilder {
    // TODO
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
    fn accept_route_state(&self, ctx: &mut RouteContext);

    /// Accepts insertion solution context allowing to update job insertion data.
    /// This method called twice: before insertion of all jobs starts and when it ends.
    /// Please note, that it is important to update only stale routes as this allows to avoid
    /// updating non changed route states.
    fn accept_solution_state(&self, ctx: &mut SolutionContext);

    /// Returns unique constraint state keys.
    /// Used to avoid state key interference.
    fn state_keys(&self) -> Iter<i32>;
}

pub trait FeatureConstraint {
    /// Evaluates hard constraints violations.
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation>;

    /// Tries to merge two jobs taking into account common constraints.
    /// Returns a new job, if it is possible to merge them together having theoretically assignable
    /// job. Otherwise returns violation error code.
    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32>;
}

pub trait FeatureObjective: Objective {
    /// Estimates a cost of insertion.
    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost;
}

/// Specifies result of hard route constraint check.
#[derive(Clone, Debug)]
pub struct ConstraintViolation {
    /// Violation code which is used as marker of specific constraint violated.
    pub code: i32,
    /// True if further insertions should not be attempted.
    pub stopped: bool,
}

pub type InsertionCost = tinyvec::TinyVec<[Cost; 6]>;

pub struct FeatureRegistry {
    constraints: Vec<Arc<dyn FeatureConstraint + Send + Sync>>,
    objectives: Vec<Vec<Arc<dyn FeatureObjective<Solution = InsertionContext> + Send + Sync>>>,
    states: Vec<Arc<dyn FeatureState + Send + Sync>>,
}

impl FeatureRegistry {
    /// Creates a new instance of `FeatureRegistry` from hierarchy of the features.
    /// Hierarchy of the features should be the same as the desired objective hierarchy.
    pub fn new(prioritized_features: &[Vec<Feature>]) -> Result<Self, String> {
        let objectives = prioritized_features
            .iter()
            .map(|features| features.iter().filter_map(|feature| feature.objective.clone()).collect())
            .collect();

        let features: Vec<Feature> = prioritized_features.iter().flatten().cloned().collect();

        features.iter().filter_map(|feature| feature.state.as_ref()).flat_map(|state| state.state_keys()).try_fold(
            HashSet::<i32>::default(),
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
    pub fn merge_constrained(&self, source: Job, candidate: Job) -> Result<Job, i32> {
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
                .map(|(a, b)| a + b)
                .collect()
        })
    }
}
