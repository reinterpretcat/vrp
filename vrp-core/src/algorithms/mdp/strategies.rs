use super::*;

/// Applies q-learning strategy to calculate values for taken actions.
pub struct QLearning {
    alpha: f64,
    gamma: f64,
    initial: f64,
}

impl<S: State> LearningStrategy<S> for QLearning {
    fn value(
        &self,
        reward_value: f64,
        old_value: Option<f64>,
        next_actions_values: Option<&HashMap<S::Action, f64>>,
    ) -> f64 {
        let max_next = next_actions_values
            .and_then(|av| av.values().max_by(|a, b| a.partial_cmp(b).unwrap()).cloned())
            .unwrap_or(self.initial);

        let value = old_value.unwrap_or(self.initial);

        value + self.alpha * (reward_value + self.gamma * max_next - value)
    }
}

/// An e-greedy action selection strategy which acts as greedy except it can select some
/// random action with probability specified.
pub struct EpsilonGreedy {}

impl<S: State> ActionStrategy<S> for EpsilonGreedy {
    fn select(&self, _actions_values: &HashMap<S::Action, f64>) -> S::Action {
        unimplemented!()
    }
}
