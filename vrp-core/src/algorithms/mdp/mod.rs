//! This module contains definition of Markov Decision Process (MDP) model and related reinforcement
//! learning logic.

mod simulator;
pub use self::simulator::*;

mod strategies;
pub use self::strategies::*;

use crate::utils::{compare_floats, Random};
use hashbrown::HashMap;
use std::cmp::Ordering;
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
    fn get_actions(&self, state: &S) -> ActionEstimates<S>;

    /// Takes the action in the current agent's state. Potentially, changes agent state.
    fn take_action(&mut self, action: &S::Action);
}

/// A learning strategy for the MDP.
pub trait LearningStrategy<S: State> {
    /// Estimates an action value given received reward, current value, and actions values from the new state.
    fn value(&self, reward_value: f64, old_value: f64, estimates: &ActionEstimates<S>) -> f64;
}

/// A policy strategy for MDP.
pub trait PolicyStrategy<S: State> {
    /// Selects an action from the estimated actions.
    fn select(&self, estimates: &ActionEstimates<S>) -> Option<S::Action>;
}

/// Keeps track of action estimation.
pub struct ActionEstimates<S: State> {
    estimates: HashMap<S::Action, f64>,
    max: Option<(S::Action, f64)>,
    min: Option<(S::Action, f64)>,
}

impl<S: State> ActionEstimates<S> {
    /// Sets estimate for given action.
    pub fn insert(&mut self, action: <S as State>::Action, estimate: f64) {
        self.estimates.insert(action.clone(), estimate);

        // TODO optimize to avoid loops?
        let (min, max) = Self::get_min_max(&self.estimates);
        self.min = min;
        self.max = max;
    }

    /// Returns an action based on its estimate interpreted as weight.
    pub fn weighted(&self, random: &(dyn Random + Send + Sync)) -> Option<S::Action> {
        // NOTE algorithm below doesn't work with negative values and zeros
        let offset = match self.min {
            Some((_, value)) if compare_floats(value, 0.) == Ordering::Less => -value + 0.01,
            Some((_, value)) if compare_floats(value, 0.) == Ordering::Equal => 0.01,
            _ => 0.,
        };

        self.estimates
            .iter()
            .map(|(action, value)| {
                let value = value + offset;
                (-random.uniform_real(0., 1.).ln() / value, action)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, action)| action.clone())
    }

    /// Gets random action.
    pub fn random(&self, random: &(dyn Random + Send + Sync)) -> Option<S::Action> {
        let random_idx = random.uniform_int(0, self.estimates.len() as i32 - 1) as usize;
        self.estimates.keys().nth(random_idx).cloned()
    }

    /// Returns a max estimate.
    pub fn max_estimate(&self) -> Option<(S::Action, f64)> {
        self.max.clone()
    }

    /// Returns a min estimate.
    pub fn min_estimate(&self) -> Option<(S::Action, f64)> {
        self.min.clone()
    }

    /// Returns actual action estimates data.
    pub fn data(&self) -> &HashMap<S::Action, f64> {
        &self.estimates
    }

    fn get_min_max(map: &HashMap<S::Action, f64>) -> (Option<(S::Action, f64)>, Option<(S::Action, f64)>) {
        let max = map.iter().max_by(|(_, a), (_, b)| compare_floats(**a, **b)).map(|(a, b)| (a.clone(), *b));
        let min = map.iter().min_by(|(_, a), (_, b)| compare_floats(**a, **b)).map(|(a, b)| (a.clone(), *b));

        (min, max)
    }
}

impl<S: State> Default for ActionEstimates<S> {
    fn default() -> Self {
        Self { estimates: Default::default(), max: None, min: None }
    }
}

impl<S: State> Clone for ActionEstimates<S> {
    fn clone(&self) -> Self {
        let estimates = self.estimates.iter().map(|(s, v)| (s.clone(), *v)).collect::<HashMap<_, _>>();
        ActionEstimates::from(estimates)
    }
}

impl<S: State> From<HashMap<S::Action, f64>> for ActionEstimates<S> {
    fn from(map: HashMap<<S as State>::Action, f64>) -> Self {
       let (min, max) = Self::get_min_max(&map);

        Self { estimates: map, max, min }
    }
}

impl<S: State> Into<HashMap<S::Action, f64>> for ActionEstimates<S> {
    fn into(self) -> HashMap<S::Action, f64> {
        self.estimates
    }
}
