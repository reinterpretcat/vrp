use super::*;
use crate::helpers::utils::create_test_random;
use crate::utils::{random_argmax, DefaultDistributionSampler};

#[test]
fn can_find_proper_estimations() {
    let total_episodes = 100;
    let expected_failures_threshold = 20;
    let failed_slot_estimations: usize = (0..total_episodes)
        .map(|_| {
            let slot_means = &[5.0_f64, 9., 7., 13., 11.];
            let slot_vars = &[2.0_f64, 3., 4., 6., 1.];
            let prior_mean = 1.;
            let attempts = 1000;
            let delta = 2.;

            let random = create_test_random();
            let sampler = DefaultDistributionSampler::new(random.clone());
            let mut slots = (0..5)
                .map(|idx| {
                    SlotMachine::new(prior_mean, sampler.clone(), {
                        let sampler = sampler.clone();
                        move || sampler.normal(slot_means[idx], slot_vars[idx].sqrt())
                    })
                })
                .collect::<Vec<_>>();

            for _ in 0..attempts {
                let slot_idx = random_argmax(slots.iter().map(|slot| slot.sample()), random.as_ref()).unwrap();
                slots[slot_idx].play();
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

    assert!(failed_slot_estimations < expected_failures_threshold);
}
