#[cfg(test)]
#[path = "../../../tests/unit/algorithms/mdp/simulator_test.rs"]
mod simulator_test;

use super::*;
use crate::utils::{parallel_into_collect, CollectGroupBy, Parallelism};

/// A type which keeps track of all state-action estimates.
pub type StateEstimates<S> = HashMap<S, ActionEstimates<S>>;

/// A simulator to train agent with multiple episodes.
pub struct Simulator<S: State> {
    q: StateEstimates<S>,
    learning_strategy: Box<dyn LearningStrategy<S> + Send + Sync>,
    policy_strategy: Box<dyn PolicyStrategy<S> + Send + Sync>,
}

impl<S: State> Simulator<S> {
    /// Creates a new instance of MDP simulator.
    pub fn new(
        learning_strategy: Box<dyn LearningStrategy<S> + Send + Sync>,
        policy_strategy: Box<dyn PolicyStrategy<S> + Send + Sync>,
    ) -> Self {
        Self { q: Default::default(), learning_strategy, policy_strategy }
    }

    /// Return a learned optimal policy for given state.
    pub fn get_optimal_policy(&self, state: &S) -> Option<(<S as State>::Action, f64)> {
        self.q.get(state).and_then(|estimates| {
            let strategy: Box<dyn PolicyStrategy<S>> = Box::<Greedy>::default();
            strategy
                .select(estimates)
                .and_then(|action| estimates.data().get(&action).map(|estimate| (action, *estimate)))
        })
    }

    /// Gets state estimates.
    pub fn get_state_estimates(&self) -> &StateEstimates<S> {
        &self.q
    }

    /// Sets action estimates for given state.
    pub fn set_action_estimates(&mut self, state: S, estimates: ActionEstimates<S>) {
        self.q.insert(state, estimates);
    }

    /// Sets a new learning strategy.
    pub fn set_learning_strategy(&mut self, learning_strategy: Box<dyn LearningStrategy<S> + Send + Sync>) {
        self.learning_strategy = learning_strategy;
    }

    /// Sets a new policy strategy.
    pub fn set_policy_strategy(&mut self, policy_strategy: Box<dyn PolicyStrategy<S> + Send + Sync>) {
        self.policy_strategy = policy_strategy;
    }

    /// Runs single episode for each of the given agents in parallel.
    pub fn run_episodes<A>(
        &mut self,
        agents: Vec<Box<A>>,
        parallelism: Parallelism,
        reducer: impl Fn(&S, &[f64]) -> f64,
    ) -> Vec<Box<A>>
    where
        A: Agent<S> + Send + Sync,
    {
        let (agents, qs): (Vec<_>, Vec<_>) =
            parallel_into_collect(agents.into_iter().enumerate().collect(), |(idx, agent)| {
                let mut agent = agent;
                parallelism.thread_pool_execute(idx, || {
                    let qs = Self::run_episode(
                        agent.as_mut(),
                        self.learning_strategy.as_ref(),
                        self.policy_strategy.as_ref(),
                        &self.q,
                    );
                    (agent, qs)
                })
            })
            .into_iter()
            .unzip();

        merge_vec_maps(qs, |(state, values)| {
            let action_values = self.q.entry(state.clone()).or_insert_with(ActionEstimates::default);
            let vec_map = values.into_iter().map(|estimates| estimates.into()).collect();
            merge_vec_maps(vec_map, |(action, values)| {
                action_values.insert(action, reducer(&state, values.as_slice()));
            });
            action_values.recalculate_min_max();
        });

        agents
    }

    fn run_episode(
        agent: &mut dyn Agent<S>,
        learning_strategy: &(dyn LearningStrategy<S> + Send + Sync),
        policy_strategy: &(dyn PolicyStrategy<S> + Send + Sync),
        q: &StateEstimates<S>,
    ) -> StateEstimates<S> {
        let mut q_new = StateEstimates::new();

        loop {
            let old_state = agent.get_state().clone();
            Self::ensure_actions(&mut q_new, q, &old_state, agent);
            let old_estimates = q_new.get(&old_state).unwrap();

            let action = policy_strategy.select(old_estimates);
            if action.is_none() {
                break;
            }

            let action = action.unwrap();
            agent.take_action(&action);
            let old_value = *old_estimates.data().get(&action).unwrap();

            let next_state = agent.get_state();
            let reward_value = next_state.reward();

            Self::ensure_actions(&mut q_new, q, next_state, agent);
            let new_estimates = q_new.get(next_state).unwrap();
            let new_value = learning_strategy.value(reward_value, old_value, new_estimates);

            q_new.entry(old_state.clone()).and_modify(|estimates| {
                estimates.insert(action.clone(), new_value);
                estimates.recalculate_min_max();
            });
        }

        q_new
    }

    fn ensure_actions(q_new: &mut StateEstimates<S>, q: &StateEstimates<S>, state: &S, agent: &dyn Agent<S>) {
        match (q_new.get(state), q.get(state)) {
            (None, Some(estimates)) => q_new.insert(state.clone(), estimates.clone()),
            (None, None) => q_new.insert(state.clone(), agent.get_actions(state)),
            (Some(_), _) => None,
        };
    }
}

fn merge_vec_maps<K: Eq + Hash, V, F: FnMut((K, Vec<V>))>(vec_map: Vec<HashMap<K, V>>, merge_func: F) {
    vec_map.into_iter().flat_map(|q| q.into_iter()).collect_group_by().into_iter().for_each(merge_func)
}
