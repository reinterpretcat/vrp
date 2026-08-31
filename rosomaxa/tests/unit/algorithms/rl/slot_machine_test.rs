use super::*;
use crate::helpers::utils::create_test_random;
use crate::utils::DefaultDistributionSampler;

#[derive(Clone)]
struct TestAction;

impl SlotAction for TestAction {
    type Context = bool;
    type Feedback = TestFeedback;

    fn take(&self, success: Self::Context) -> Self::Feedback {
        TestFeedback(success)
    }
}

struct TestFeedback(bool);

impl SlotFeedback for TestFeedback {
    fn is_success(&self) -> bool {
        self.0
    }
}

#[derive(Clone)]
struct MeanDistributionSampler;

impl DistributionSampler for MeanDistributionSampler {
    fn gamma(&self, shape: Float, _: Float) -> Float {
        shape
    }

    fn normal(&self, mean: Float, _: Float) -> Float {
        mean
    }
}

#[test]
fn can_use_success_prior_as_specified() {
    let slot = SlotMachine::new(2.5, TestAction, MeanDistributionSampler);

    assert_eq!(slot.get_params().alpha, 2.5);
    assert_eq!(slot.sample(), 2.5 / 3.5);
}

#[test]
fn can_reset_learning_state_and_keep_usage() {
    let mut slot = SlotMachine::new(2.5, TestAction, MeanDistributionSampler);

    slot.update(&TestFeedback(false));
    slot.reset();

    let BernoulliParams { alpha, beta, mean, variance, observations } = slot.get_params();
    assert_eq!((alpha, beta, mean, observations), (2.5, PRIOR_BETA, 2.5 / 3.5, 1));
    assert!((variance - 2.5 / (3.5_f64.powi(2) * 4.5)).abs() < 1e-12);
    assert_eq!(slot.sample(), 2.5 / 3.5);
}

#[test]
fn can_update_beta_posterior() {
    let mut slot = SlotMachine::new(1., TestAction, MeanDistributionSampler);

    slot.update(&TestFeedback(true));

    let BernoulliParams { alpha, beta, mean, variance, observations } = slot.get_params();
    assert_eq!((alpha, beta, mean, observations), (2., 1., 2. / 3., 1));
    assert!((variance - 1. / 18.).abs() < 1e-12);
}

#[test]
fn can_limit_confidence_and_adapt_to_regime_change() {
    let mut slot = SlotMachine::new(1., TestAction, MeanDistributionSampler);

    (0..1_000).for_each(|_| slot.update(&TestFeedback(true)));
    let successful = slot.get_params();
    assert!(successful.alpha + successful.beta <= MAX_EVIDENCE + f64::EPSILON);
    assert!(successful.mean > 0.99);

    (0..1_000).for_each(|_| slot.update(&TestFeedback(false)));
    let unsuccessful = slot.get_params();
    assert!(unsuccessful.alpha + unsuccessful.beta <= MAX_EVIDENCE + f64::EPSILON);
    assert!(unsuccessful.mean < 0.01);
}

#[test]
fn can_keep_sampling_numerically_stable() {
    let sampler = DefaultDistributionSampler::new(create_test_random());
    let mut slot = SlotMachine::new(1., TestAction, sampler);

    (0..200_000).for_each(|_| slot.update(&TestFeedback(false)));
    assert!(slot.sample().is_finite());

    (0..1_000).for_each(|_| slot.update(&TestFeedback(true)));
    assert!(slot.sample().is_finite());
    assert!(slot.get_params().mean > 0.99);
}

#[test]
fn can_play_action() {
    let slot = SlotMachine::new(1., TestAction, MeanDistributionSampler);

    assert!(slot.play(true).is_success());
    assert!(!slot.play(false).is_success());
}
