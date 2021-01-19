use super::*;

/// Applies q-learning strategy to calculate values for taken actions.
pub struct QLearning {
    alpha: f64,
    gamma: f64,
    initial: f64,
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
pub struct EpsilonGreedy {}

impl<S: State> PolicyStrategy<S> for EpsilonGreedy {
    fn select(&self, _estimates: &ActionsEstimate<S>) -> S::Action {
        unimplemented!()
    }
}
