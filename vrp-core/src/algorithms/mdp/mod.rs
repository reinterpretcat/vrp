//! This module contains definition of Markov Decision Process (MDP) model and related reinforcement
//! learning logic.

mod simulator;
pub use self::simulator::*;

mod strategies;
pub use self::strategies::*;

use crate::utils::compare_floats;
use hashbrown::HashMap;
use std::hash::Hash;

/// Represents a state in MDP.
pub trait State: Clone + Hash + Eq + Send + Sync {
    /// Action type associated with the state.
    type Action: Clone + Hash + Eq + Send + Sync;

    /// Returns reward to be in this state.
    fn reward(&self) -> f64;
}

/// Represents an agent in MDP.
pub trait Agent<S: State> {
    /// Returns the current state of the agent.
    fn get_state(&self) -> &S;

    /// Returns agent's actions for given state with their estimates. If no actions are
    /// associated, then the state is considered as terminal.
    fn get_actions(&self, state: &S) -> ActionsEstimate<S>;

    /// Takes the action in the current agent's state. Potentially, changes agent state.
    fn take_action(&mut self, action: &S::Action);
}

/// A learning strategy for the MDP.
pub trait LearningStrategy<S: State> {
    /// Estimates an action value given received reward, current value, and actions values from the new state.
    fn value(&self, reward_value: f64, old_value: f64, estimates: &ActionsEstimate<S>) -> f64;
}

/// A policy strategy for MDP.
pub trait PolicyStrategy<S: State> {
    /// Selects an action from the estimated actions.
    fn select(&self, estimates: &ActionsEstimate<S>) -> Option<S::Action>;
}

/// Keeps track of action estimation.
#[derive(Clone)]
pub struct ActionsEstimate<S: State> {
    estimations: HashMap<S::Action, f64>,
    max_estimate: Option<f64>,
    min_estimate: Option<f64>,
}

impl<S: State> ActionsEstimate<S> {
    /// Sets estimate for given action.
    pub fn insert(&mut self, action: <S as State>::Action, estimate: f64) {
        self.estimations.insert(action, estimate);

        self.max_estimate = self.max_estimate.map(|old| old.max(estimate)).or(Some(estimate));
        self.min_estimate = self.min_estimate.map(|old| old.min(estimate)).or(Some(estimate));
    }

    /// Returns an action based on its estimate interpreted as weight.
    pub fn weighted(&self) -> <S as State>::Action {
        unimplemented!()
    }

    /// Returns a max estimate.
    pub fn max_estimate(&self) -> Option<f64> {
        self.max_estimate
    }

    /// Returns a min estimate.
    pub fn min_estimate(&self) -> Option<f64> {
        self.min_estimate
    }

    /// Returns actual estimation data.
    pub fn data(&self) -> &HashMap<S::Action, f64> {
        &self.estimations
    }
}

impl<S: State> Default for ActionsEstimate<S> {
    fn default() -> Self {
        Self { estimations: Default::default(), max_estimate: None, min_estimate: None }
    }
}

impl<S: State> From<HashMap<S::Action, f64>> for ActionsEstimate<S> {
    fn from(map: HashMap<<S as State>::Action, f64>) -> Self {
        let max_estimate = map.values().max_by(|a, b| compare_floats(**a, **b)).cloned();
        let min_estimate = map.values().min_by(|a, b| compare_floats(**a, **b)).cloned();

        Self { estimations: map, max_estimate, min_estimate }
    }
}

impl<S: State> Into<HashMap<S::Action, f64>> for ActionsEstimate<S> {
    fn into(self) -> HashMap<S::Action, f64> {
        self.estimations
    }
}
