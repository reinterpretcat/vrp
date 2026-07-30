use super::*;
use crate::helpers::utils::create_test_random;
use crate::utils::DefaultDistributionSampler;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
struct TestAction(DefaultDistributionSampler);

impl SlotAction for TestAction {
    type Context = (Float, Float);
    type Feedback = TestFeedback;

    fn take(&self, context: Self::Context) -> Self::Feedback {
        let (mean, var) = context;
        let reward = self.0.normal(mean, var.sqrt());

        TestFeedback(reward)
    }
}

struct TestFeedback(Float);

impl SlotFeedback for TestFeedback {
    fn reward(&self) -> Float {
        self.0
    }
}

#[derive(Clone, Default)]
struct TestDistributionSampler {
    last_std_dev: Rc<Cell<Float>>,
}

impl DistributionSampler for TestDistributionSampler {
    fn gamma(&self, _: Float, _: Float) -> Float {
        1.
    }

    fn normal(&self, mean: Float, std_dev: Float) -> Float {
        self.last_std_dev.set(std_dev);
        mean
    }
}

#[derive(Clone)]
struct OptimisticDistributionSampler;

impl DistributionSampler for OptimisticDistributionSampler {
    fn gamma(&self, shape: Float, scale: Float) -> Float {
        shape * scale
    }

    fn normal(&self, mean: Float, std_dev: Float) -> Float {
        mean + std_dev
    }
}

#[test]
fn can_use_prior_mean_as_specified() {
    let sampler = DefaultDistributionSampler::new(create_test_random());
    let slot = SlotMachine::new(2.5, TestAction(sampler.clone()), sampler);

    assert_eq!(slot.get_params().2, 2.5);
}

#[test]
fn can_sample_posterior_mean_using_effective_count() {
    let action_sampler = DefaultDistributionSampler::new(create_test_random());
    let sampler = TestDistributionSampler::default();
    let mut slot = SlotMachine::new(1., TestAction(action_sampler), sampler.clone());

    slot.update(&TestFeedback(1.));
    slot.sample();

    let alpha = slot.get_params().0;
    let expected_std_dev = (1. / (2. * alpha)).sqrt();
    assert!((sampler.last_std_dev.get() - expected_std_dev).abs() < f64::EPSILON);

    let initial_std_dev = sampler.last_std_dev.get();
    (0..100).for_each(|_| slot.update(&TestFeedback(1.)));
    slot.sample();

    assert!(sampler.last_std_dev.get() < initial_std_dev);
}

#[test]
fn can_prefer_mean_over_observation_variance() {
    let action_sampler = DefaultDistributionSampler::new(create_test_random());
    let sampler = OptimisticDistributionSampler;
    let mut stable = SlotMachine::new(1., TestAction(action_sampler.clone()), sampler.clone());
    let mut noisy = SlotMachine::new(1., TestAction(action_sampler), sampler);

    (0..200).for_each(|idx| {
        stable.update(&TestFeedback(1.));
        noisy.update(&TestFeedback(if idx % 2 == 0 { -2. } else { 2. }));
    });

    assert!(stable.mu > noisy.mu);
    assert!(stable.sample() > noisy.sample());
}

#[test]
fn can_adapt_to_reward_regime_change() {
    let action_sampler = DefaultDistributionSampler::new(create_test_random());
    let mut slot = SlotMachine::new(0., TestAction(action_sampler), OptimisticDistributionSampler);

    (0..200).for_each(|_| slot.update(&TestFeedback(1.)));
    assert!(slot.mu > 0.9);

    (0..500).for_each(|_| slot.update(&TestFeedback(-1.)));
    assert!(slot.mu < -0.8);
}

#[test]
fn can_find_proper_estimations() {
    let sockets = 5;
    let total_episodes = 100;
    let expected_failures_threshold = (0.3 * (sockets * total_episodes) as Float) as usize;
    let failed_slot_estimations: usize = (0..total_episodes)
        .map(|_| {
            let slot_means: &[Float; 5] = &[5., 9., 7., 13., 11.];
            let slot_vars: &[Float; 5] = &[2., 3., 4., 6., 1.];
            let prior_mean = 1.;
            let attempts_per_slot = 1000;
            let delta = 2.;

            let random = create_test_random();
            let sampler = DefaultDistributionSampler::new(random.clone());
            let mut slots = (0..sockets)
                .map(|_| SlotMachine::new(prior_mean, TestAction(sampler.clone()), sampler.clone()))
                .collect::<Vec<_>>();

            // Play each slot independently to test estimation convergence
            for slot_idx in 0..sockets {
                for _ in 0..attempts_per_slot {
                    let slot = &mut slots[slot_idx];
                    let feedback = slot.play((slot_means[slot_idx], slot_vars[slot_idx]));
                    slot.update(&feedback);
                }
            }

            slots
                .iter()
                .enumerate()
                .filter(|(idx, slot)| {
                    (slot.mu - slot_means[*idx]).abs() > delta || (slot.v - slot_vars[*idx]).abs() > delta
                })
                .map(|_| 1)
                .sum::<usize>()
        })
        .sum();

    if failed_slot_estimations > expected_failures_threshold {
        panic!("too many estimation failures: {failed_slot_estimations} < {expected_failures_threshold}")
    }
}
