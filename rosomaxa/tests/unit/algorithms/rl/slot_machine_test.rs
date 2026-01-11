use super::*;
use crate::helpers::utils::create_test_random;
use crate::utils::DefaultDistributionSampler;

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
