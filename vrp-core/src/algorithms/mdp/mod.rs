//! This module contains definition of Markov Decision Process (MDP) model and related reinforcement
//! learning logic.

mod simulator;
pub use self::simulator::*;

mod strategies;
pub use self::strategies::*;

use std::collections::HashMap;
use std::hash::Hash;

/// Keeps track of action estimation.
pub type ActionsEstimate<S> = HashMap<<S as State>::Action, f64>;

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
    fn select(&self, estimates: &ActionsEstimate<S>) -> S::Action;
}
