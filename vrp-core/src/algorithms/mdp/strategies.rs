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
    fn value(&self, reward_value: f64, old_value: f64, estimates: &ActionsEstimate<S>) -> f64 {
        let next_max = estimates.values().max_by(|a, b| a.partial_cmp(b).unwrap()).cloned().unwrap_or(0.);

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
    fn value(&self, reward_value: f64, old_value: f64, _estimates: &ActionsEstimate<S>) -> f64 {
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
    fn select(&self, estimates: &ActionsEstimate<S>) -> Option<S::Action> {
        if estimates.is_empty() {
            return None;
        }

        if self.random.is_hit(self.epsilon) {
            let random_idx = self.random.uniform_int(0, estimates.len() as i32 - 1) as usize;
            estimates.keys().nth(random_idx).cloned()
        } else {
            estimates.iter().max_by(|(_, x), (_, y)| compare_floats(**x, **y)).map(|(a, _)| a.clone())
        }
    }
}

/// A greedy strategy.
pub struct Greedy;

impl Default for Greedy {
    fn default() -> Self {
        Self {}
    }
}

impl<S: State> PolicyStrategy<S> for Greedy {
    fn select(&self, estimates: &ActionsEstimate<S>) -> Option<S::Action> {
        estimates.iter().max_by(|(_, x), (_, y)| compare_floats(**x, **y)).map(|(a, _)| a.clone())
    }
}
