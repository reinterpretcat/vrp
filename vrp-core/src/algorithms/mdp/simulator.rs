use super::*;
use crate::utils::{parallel_into_collect, CollectGroupBy};

/// A simulator to train agent with multiple episodes.
pub struct Simulator<S: State> {
    q: QType<S>,
    learning: Box<dyn LearningStrategy<S> + Send + Sync>,
    action: Box<dyn ActionStrategy<S> + Send + Sync>,
    termination: Box<dyn TerminationStrategy<S> + Send + Sync>,
}

type QType<S> = HashMap<S, HashMap<<S as State>::Action, f64>>;

impl<S: State> Simulator<S> {
    /// Creates a new instance of MDP simulator.
    pub fn new(
        learning: Box<dyn LearningStrategy<S> + Send + Sync>,
        action: Box<dyn ActionStrategy<S> + Send + Sync>,
        termination: Box<dyn TerminationStrategy<S> + Send + Sync>,
    ) -> Self {
        Self { q: Default::default(), learning, action, termination }
    }

    /// Runs multiple episodes in parallel for given actors .
    pub fn run_episodes(&mut self, agents: Vec<Box<dyn Agent<S> + Send + Sync>>) {
        let qs = parallel_into_collect(agents, |mut a| {
            Self::run_episode(
                a.as_mut(),
                self.learning.as_ref(),
                self.action.as_ref(),
                self.termination.as_ref(),
                &self.q,
            )
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
        action: &(dyn ActionStrategy<S> + Send + Sync),
        termination: &(dyn TerminationStrategy<S> + Send + Sync),
        q: &QType<S>,
    ) -> QType<S> {
        let mut q_new = QType::new();

        loop {
            let state_old = agent.get_state().clone();
            let actions_values = q_new.get(&state_old).or_else(|| q.get(&state_old));

            if actions_values.is_none() || termination.is_termination(&state_old) {
                break;
            }

            let actions_values = actions_values.unwrap();
            let action = action.select(actions_values);

            agent.take_action(&action);

            let state_next = agent.get_state();
            let reward_value = state_next.reward();

            let old_value = actions_values.get(&action).cloned();
            let new_actions_values = q_new.get(state_next).or_else(|| q.get(state_next));
            let new_value = learning.value(reward_value, old_value, new_actions_values);

            q_new.entry(state_old).or_insert_with(|| HashMap::new()).insert(action, new_value);
        }

        q_new
    }
}

fn merge_vec_maps<K: Eq + Hash, V, F: FnMut((K, Vec<V>)) -> ()>(vec_map: Vec<HashMap<K, V>>, merge_func: F) {
    vec_map.into_iter().flat_map(|q| q.into_iter()).collect_group_by().into_iter().for_each(merge_func)
}
