use crate::construction::heuristics::{MoveContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::models::*;
use rosomaxa::prelude::unwrap_from_result;
use std::slice::Iter;
use std::sync::Arc;

/// Combines multiple features as single with given name.
pub fn combine_features(name: &str, features: &[Feature]) -> Result<Feature, String> {
    let objectives = features.iter().filter_map(|feature| feature.objective.clone()).collect::<Vec<_>>();
    if objectives.len() > 1 {
        return Err(format!(
            "combination of features with multiple objective is not supported. Objective count: {}",
            objectives.len()
        ));
    }

    let constraints = features.iter().filter_map(|feature| feature.constraint.clone()).collect::<Vec<_>>();

    let states = features.iter().filter_map(|feature| feature.state.clone()).collect::<Vec<_>>();

    let feature = Feature {
        name: name.to_string(),
        constraint: if constraints.is_empty() {
            None
        } else {
            Some(Arc::new(CombinedFeatureConstraint { constraints }))
        },
        objective: objectives.first().cloned(),
        state: if states.is_empty() { None } else { Some(Arc::new(CombinedFeatureState::new(states))) },
    };

    FeatureBuilder::from_feature(feature).build()
}

struct CombinedFeatureState {
    states: Vec<Arc<dyn FeatureState + Send + Sync>>,
    state_keys: Vec<StateKey>,
}

impl CombinedFeatureState {
    pub fn new(states: Vec<Arc<dyn FeatureState + Send + Sync>>) -> Self {
        let state_keys = states.iter().flat_map(|state| state.state_keys().cloned()).collect();
        Self { states, state_keys }
    }
}

impl FeatureState for CombinedFeatureState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, route_index: usize, job: &Job) {
        self.states.iter().for_each(|state| state.accept_insertion(solution_ctx, route_index, job));
    }

    fn accept_route_state(&self, route_ctx: &mut RouteContext) {
        self.states.iter().for_each(|state| state.accept_route_state(route_ctx));
    }

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        self.states.iter().for_each(|state| state.accept_solution_state(solution_ctx));
    }

    fn state_keys(&self) -> Iter<StateKey> {
        self.state_keys.iter()
    }
}

struct CombinedFeatureConstraint {
    constraints: Vec<Arc<dyn FeatureConstraint + Send + Sync>>,
}

impl FeatureConstraint for CombinedFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        unwrap_from_result(self.constraints.iter().try_fold(None, |_, constraint| {
            constraint.evaluate(move_ctx).map(|violation| Err(Some(violation))).unwrap_or_else(|| Ok(None))
        }))
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        self.constraints.iter().try_fold(source, |acc, constraint| constraint.merge(acc, candidate.clone()))
    }
}
