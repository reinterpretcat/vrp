#[cfg(test)]
#[path = "../../../tests/unit/algorithms/mdp/simulator_test.rs"]
mod simulator_test;

use super::*;
use crate::utils::{parallel_into_collect, CollectGroupBy};

/// A simulator to train agent with multiple episodes.
pub struct Simulator<S: State> {
    q: QType<S>,
    learning: Box<dyn LearningStrategy<S> + Send + Sync>,
    policy: Box<dyn PolicyStrategy<S> + Send + Sync>,
}

type QType<S> = HashMap<S, HashMap<<S as State>::Action, f64>>;

impl<S: State> Simulator<S> {
    /// Creates a new instance of MDP simulator.
    pub fn new(
        learning: Box<dyn LearningStrategy<S> + Send + Sync>,
        policy: Box<dyn PolicyStrategy<S> + Send + Sync>,
    ) -> Self {
        Self { q: Default::default(), learning, policy }
    }

    /// Runs single episode for each of the given agents in parallel.
    pub fn run_episodes(&mut self, agents: Vec<Box<dyn Agent<S> + Send + Sync>>) {
        let qs = parallel_into_collect(agents, |mut a| {
            Self::run_episode(a.as_mut(), self.learning.as_ref(), self.policy.as_ref(), &self.q)
        });

        merge_vec_maps(qs, |(state, values)| {
            let action_values = self.q.entry(state).or_insert_with(|| HashMap::new());
            merge_vec_maps(values, |(action, values)| {
                // TODO is there something better than average?
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                action_values.insert(action, avg);
            });
        });
    }

    fn run_episode(
        agent: &mut dyn Agent<S>,
        learning: &(dyn LearningStrategy<S> + Send + Sync),
        policy: &(dyn PolicyStrategy<S> + Send + Sync),
        q: &QType<S>,
    ) -> QType<S> {
        let mut q_new = QType::new();

        loop {
            let old_state = agent.get_state().clone();
            let old_estimates = if let Some(estimates) = q_new.get(&old_state) {
                estimates
            } else {
                q.get(&old_state)
                    .unwrap_or_else(|| q_new.entry(old_state.clone()).or_insert(agent.get_actions(&old_state)))
            };

            if old_estimates.is_empty() {
                break;
            }

            let action = policy.select(old_estimates);
            agent.take_action(&action);
            let old_value = old_estimates[&action];

            let next_state = agent.get_state();
            let reward_value = next_state.reward();

            // TODO cannot extract it as a function due to borrow checker
            let new_estimates = if let Some(estimates) = q_new.get(&next_state) {
                estimates
            } else {
                q.get(&next_state)
                    .unwrap_or_else(|| q_new.entry(next_state.clone()).or_insert(agent.get_actions(&next_state)))
            };

            let new_value = learning.value(reward_value, old_value, new_estimates);

            q_new.entry(old_state).and_modify(|estimates| {
                estimates.insert(action, new_value);
            });
        }

        q_new
    }

    // TODO add policy method
}

fn merge_vec_maps<K: Eq + Hash, V, F: FnMut((K, Vec<V>)) -> ()>(vec_map: Vec<HashMap<K, V>>, merge_func: F) {
    vec_map.into_iter().flat_map(|q| q.into_iter()).collect_group_by().into_iter().for_each(merge_func)
}
