use super::*;
use crate::utils::{compare_floats, Random};
use std::sync::Arc;

/// Applies q-learning strategy to calculate values for taken actions.
pub struct QLearning {
    alpha: f64,
    gamma: f64,
    initial: f64,
}

impl QLearning {
    pub fn new(alpha: f64, gamma: f64, initial: f64) -> Self {
        Self { alpha, gamma, initial }
    }
}

impl<S: State> LearningStrategy<S> for QLearning {
    fn value(&self, reward_value: f64, old_value: Option<f64>, estimates: Option<&ActionsEstimate<S>>) -> f64 {
        let next_max = estimates
            .and_then(|av| av.values().max_by(|a, b| a.partial_cmp(b).unwrap()).cloned())
            .unwrap_or(self.initial);

        let value = old_value.unwrap_or(self.initial);

        value + self.alpha * (reward_value + self.gamma * next_max - value)
    }
}

/// An e-greedy action selection strategy which acts as greedy except it can select some
/// random action with probability specified.
pub struct EpsilonGreedy {
    epsilon: f64,
    random: Arc<dyn Random + Send + Sync>,
}

impl EpsilonGreedy {
    pub fn new(epsilon: f64, random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { epsilon, random }
    }
}

impl<S: State> PolicyStrategy<S> for EpsilonGreedy {
    fn select(&self, estimates: &ActionsEstimate<S>) -> S::Action {
        if self.random.is_hit(self.epsilon) {
            let random_idx = self.random.uniform_int(0, estimates.len() as i32 - 1) as usize;
            estimates.keys().skip(random_idx).next().unwrap().clone()
        } else {
            estimates.iter().max_by(|(_, x), (_, y)| compare_floats(**x, **y)).unwrap().0.clone()
        }
    }
}
