//! This module provides some helper functionality to combine and use multiple features together.

use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::Job;
use crate::models::*;
use rosomaxa::prelude::*;
use std::ops::ControlFlow;
use std::sync::Arc;

/// Specifies a type for injecting custom objective combination logic.
pub type ObjectiveCombinator = dyn Fn(&[(&str, Arc<dyn FeatureObjective>)]) -> Arc<dyn FeatureObjective>;

/// Provides the way to group multiple features having more fine grained control over result.
#[derive(Default)]
pub struct FeatureCombinator {
    name: Option<String>,
    objective_combinator: Option<Box<ObjectiveCombinator>>,
    features: Vec<Feature>,
}

impl FeatureCombinator {
    /// Gives common feature name.
    pub fn use_name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds feature for combination.
    pub fn add_feature(mut self, feature: Feature) -> Self {
        self.features.push(feature);
        self
    }

    /// Adds many features for combination.
    pub fn add_features(mut self, features: &[Feature]) -> Self {
        self.features.extend(features.iter().cloned());

        self
    }

    /// Provides custom objective combinator
    pub fn use_objective_combinator(mut self, objective_combinator: Box<ObjectiveCombinator>) -> Self {
        self.objective_combinator = Some(objective_combinator);
        self
    }

    /// Builds single feature for many.
    pub fn combine(self) -> Result<Feature, GenericError> {
        let name =
            self.name.unwrap_or_else(|| self.features.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join("+"));
        let objective_combinator = self.objective_combinator.unwrap_or_else(|| {
            Box::new(|objectives| {
                let objectives = objectives.iter().map(|(_, o)| o.clone()).collect::<Vec<_>>();
                Arc::new(CombinedFeatureObjective { objectives })
            })
        });

        combine_features(name.as_ref(), self.features.as_slice(), &objective_combinator)
    }
}

/// Combines multiple features as single with given name.
fn combine_features(
    name: &str,
    features: &[Feature],
    objective_combinator: &ObjectiveCombinator,
) -> Result<Feature, GenericError> {
    let objectives = features
        .iter()
        .filter_map(|feature| Some(feature.name.as_str()).zip(feature.objective.clone()))
        .collect::<Vec<_>>();
    let objective = match objectives.len() {
        0 => None,
        1 => objectives.first().map(|(_, o)| o.clone()),
        _ => Some(objective_combinator(objectives.as_slice())),
    };

    let constraints = features.iter().filter_map(|feature| feature.constraint.clone()).collect::<Vec<_>>();
    let constraint: Option<Arc<dyn FeatureConstraint>> = match constraints.len() {
        0 => None,
        1 => constraints.first().cloned(),
        _ => Some(Arc::new(CombinedFeatureConstraint { constraints })),
    };

    let states = features.iter().filter_map(|feature| feature.state.clone()).collect::<Vec<_>>();
    let state: Option<Arc<dyn FeatureState>> = match states.len() {
        0 => None,
        1 => states.first().cloned(),
        // TODO: combined feature state seems behave differently: check reload with one state wrapped in CombinedFeatureState
        _ => Some(Arc::new(CombinedFeatureState::new(states))),
    };

    let feature = Feature { name: name.to_string(), constraint, objective, state };

    FeatureBuilder::from_feature(feature).build()
}

struct CombinedFeatureState {
    states: Vec<Arc<dyn FeatureState>>,
}

impl CombinedFeatureState {
    pub fn new(states: Vec<Arc<dyn FeatureState>>) -> Self {
        Self { states }
    }
}

impl FeatureState for CombinedFeatureState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        accept_insertion_with_states(&self.states, solution_ctx, route_index, job)
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        accept_route_state_with_states(&self.states, route_ctx)
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        accept_solution_state_with_states(&self.states, solution_ctx)
    }
}

struct CombinedFeatureConstraint {
    constraints: Vec<Arc<dyn FeatureConstraint>>,
}

impl FeatureConstraint for CombinedFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        evaluate_with_constraints(&self.constraints, move_ctx)
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        merge_with_constraints(&self.constraints, source, candidate)
    }
}

struct CombinedFeatureObjective {
    objectives: Vec<Arc<dyn FeatureObjective>>,
}

impl FeatureObjective for CombinedFeatureObjective {
    fn fitness(&self, solution: &InsertionContext) -> f64 {
        // NOTE: just summing all objective values together
        self.objectives.iter().map(|o| o.fitness(solution)).sum::<Cost>()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        self.objectives.iter().map(|o| o.estimate(move_ctx)).sum::<Cost>()
    }
}

pub(crate) fn notify_failure_with_states(
    states: &[Arc<dyn FeatureState>],
    solution_ctx: &mut SolutionContext,
    route_indices: &[usize],
    jobs: &[Job],
) -> bool {
    // NOTE exit when first true is returned
    states.iter().any(|state| state.notify_failure(solution_ctx, route_indices, jobs))
}

pub(crate) fn accept_insertion_with_states(
    states: &[Arc<dyn FeatureState>],
    solution_ctx: &mut SolutionContext,
    route_index: usize,
    job: &Job,
) {
    let activities = solution_ctx.routes.get_mut(route_index).unwrap().route().tour.job_activity_count();
    states.iter().for_each(|state| state.accept_insertion(solution_ctx, route_index, job));
    assert_eq!(activities, solution_ctx.routes.get_mut(route_index).unwrap().route().tour.job_activity_count());
}

pub(crate) fn accept_route_state_with_states(states: &[Arc<dyn FeatureState>], route_ctx: &mut RouteContext) {
    if route_ctx.is_stale() {
        route_ctx.state_mut().clear();

        let activities = route_ctx.route().tour.job_activity_count();
        states.iter().for_each(|state| state.accept_route_state(route_ctx));
        assert_eq!(activities, route_ctx.route().tour.job_activity_count());

        route_ctx.mark_stale(false);
    }
}

pub(crate) fn accept_solution_state_with_states(states: &[Arc<dyn FeatureState>], solution_ctx: &mut SolutionContext) {
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

            states
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

pub(crate) fn merge_with_constraints(
    constraints: &[Arc<dyn FeatureConstraint>],
    source: Job,
    candidate: Job,
) -> Result<Job, ViolationCode> {
    constraints.iter().try_fold(source, |acc, constraint| constraint.merge(acc, candidate.clone()))
}

pub(crate) fn evaluate_with_constraints(
    constraints: &[Arc<dyn FeatureConstraint>],
    move_ctx: &MoveContext<'_>,
) -> Option<ConstraintViolation> {
    constraints
        .iter()
        .try_fold(None, |_, constraint| {
            constraint
                .evaluate(move_ctx)
                .map(|violation| ControlFlow::Break(Some(violation)))
                .unwrap_or_else(|| ControlFlow::Continue(None))
        })
        .unwrap_value()
}
