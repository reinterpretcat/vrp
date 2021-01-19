//! This module contains definition of Markov Decision Process (MDP) model and related reinforcement
//! learning logic.

mod simulator;
pub use self::simulator::*;

mod strategies;
pub use self::strategies::*;

use hashbrown::HashMap;
use std::hash::Hash;

/// Keeps track of action estimation.
pub type ActionsEstimate<S> = HashMap<<S as State>::Action, f64>;

/// Represents a state in MDP.
pub trait State: Clone + Hash + Eq + Send + Sync {
    /// Action type associated with the state.
    type Action: Clone + Hash + Eq + Send + Sync;

    /// Returns actions associated with the state. If no actions are associated, then
    /// the state is considered as terminal.
    fn actions(&self) -> Option<&ActionsEstimate<Self>>;

    /// Returns reward to be in this state.
    fn reward(&self) -> f64;
}

/// Represents an agent in MDP.
pub trait Agent<S: State> {
    /// Returns the current state of the agent.
    fn get_state(&self) -> &S;

    /// Takes the action in the current agent's state. Potentially, changes agent state.
    fn take_action(&mut self, action: &S::Action);
}

/// A learning strategy for the MDP.
pub trait LearningStrategy<S: State> {
    /// Estimates an action value given received reward, current value, and actions values from the new state.
    fn value(&self, reward_value: f64, old_value: Option<f64>, estimations: Option<&ActionsEstimate<S>>) -> f64;
}

/// An action selection strategy.
pub trait ActionStrategy<S: State> {
    /// Selects an action from the estimated actions.
    fn select(&self, estimations: &ActionsEstimate<S>) -> S::Action;
}

/// A termination strategy.
pub trait TerminationStrategy<S: State> {
    /// Returns true if state is terminal.
    fn is_termination(&self, state: &S) -> bool;
}
