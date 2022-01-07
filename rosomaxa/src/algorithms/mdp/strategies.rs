use super::*;
use crate::utils::{compare_floats, Random};
use std::sync::Arc;

/// Applies q-learning strategy to calculate values for taken actions.
pub struct QLearning {
    alpha: f64,
    gamma: f64,
}

impl QLearning {
    /// Creates a new instance of `QLearning`.
    pub fn new(alpha: f64, gamma: f64) -> Self {
        Self { alpha, gamma }
    }
}

impl<S: State> LearningStrategy<S> for QLearning {
    fn value(&self, reward_value: f64, old_value: f64, estimates: &ActionEstimates<S>) -> f64 {
        let next_max = estimates.max.as_ref().map_or(0., |(_, v)| *v);

        old_value + self.alpha * (reward_value + self.gamma * next_max - old_value)
    }
}

/// Applies monte cargo learning strategy to calculate values for taken actions.
pub struct MonteCarlo {
    alpha: f64,
}

impl MonteCarlo {
    /// Creates a new instance of `MonteCarlo`.
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }
}

impl<S: State> LearningStrategy<S> for MonteCarlo {
    fn value(&self, reward_value: f64, old_value: f64, _estimates: &ActionEstimates<S>) -> f64 {
        old_value + self.alpha * (reward_value - old_value)
    }
}

/// An e-greedy action selection strategy which acts as greedy except it can select some
/// random action with probability specified.
pub struct EpsilonGreedy {
    epsilon: f64,
    random: Arc<dyn Random + Send + Sync>,
}

impl EpsilonGreedy {
    /// Creates a new instance of `EpsilonGreedy`.
    pub fn new(epsilon: f64, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { epsilon, random }
    }
}

impl<S: State> PolicyStrategy<S> for EpsilonGreedy {
    fn select(&self, estimates: &ActionEstimates<S>) -> Option<S::Action> {
        if estimates.data().is_empty() {
            return None;
        }

        if self.random.is_hit(self.epsilon) {
            estimates.random(self.random.as_ref())
        } else {
            estimates.data().iter().max_by(|(_, x), (_, y)| compare_floats(**x, **y)).map(|(a, _)| a.clone())
        }
    }
}

/// A greedy strategy.
#[derive(Default)]
pub struct Greedy;

impl<S: State> PolicyStrategy<S> for Greedy {
    fn select(&self, estimates: &ActionEstimates<S>) -> Option<S::Action> {
        estimates.data().iter().max_by(|(_, x), (_, y)| compare_floats(**x, **y)).map(|(a, _)| a.clone())
    }
}

/// An e-weighted action selection strategy.
pub struct EpsilonWeighted {
    epsilon: f64,
    random: Arc<dyn Random + Send + Sync>,
}

impl EpsilonWeighted {
    /// Creates a new instance of `EpsilonWeighted`.
    pub fn new(epsilon: f64, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { epsilon, random }
    }
}

impl<S: State> PolicyStrategy<S> for EpsilonWeighted {
    fn select(&self, estimates: &ActionEstimates<S>) -> Option<S::Action> {
        if estimates.data().is_empty() {
            return None;
        }

        if self.random.is_hit(self.epsilon) {
            estimates.random(self.random.as_ref())
        } else {
            estimates.weighted(self.random.as_ref())
        }
    }
}
