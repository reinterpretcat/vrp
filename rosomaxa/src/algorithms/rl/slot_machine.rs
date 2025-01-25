#[cfg(test)]
#[path = "../../../tests/unit/algorithms/rl/slot_machine_test.rs"]
mod slot_machine_test;

use crate::utils::{DistributionSampler, Float};

/// Represents an action on slot machine;
pub trait SlotAction {
    /// An environment context.
    type Context;
    ///  A feedback from taking slot action.
    type Feedback: SlotFeedback;

    /// Take an action for given context and return reward.
    fn take(&self, context: Self::Context) -> Self::Feedback;
}

/// Provides a feedback for taking an action on a slot.
pub trait SlotFeedback {
    /// A reward for taking an action on a slot machine.
    fn reward(&self) -> Float;
}

/// Simulates a slot machine.
/// Internally tries to estimate reward probability distribution using one of methods from Thompson sampling.
#[derive(Clone)]
pub struct SlotMachine<A, S> {
    /// The number of times this slot machine has been tried.
    n: usize,
    /// Gamma shape parameter.
    alpha: Float,
    /// Gamma rate parameter.
    beta: Float,
    /// Estimated mean.
    mu: Float,
    /// Estimated variance.
    v: Float,
    /// Sampler: used to provide samples from underlying estimated distribution.
    sampler: S,
    /// Actual slot action function.
    action: A,
}

impl<A, S> SlotMachine<A, S>
where
    A: SlotAction + Clone,
    S: DistributionSampler + Clone,
{
    /// Creates a new instance of `SlotMachine`.
    pub fn new(prior_mean: Float, action: A, sampler: S) -> Self {
        let alpha = 1.;
        let beta = 10.;
        let mu = prior_mean;
        let v = beta / (alpha + 1.);

        Self { n: 0, alpha, beta, mu, v, action, sampler }
    }

    /// Samples from estimated normal distribution.
    pub fn sample(&self) -> Float {
        let precision = self.sampler.gamma(self.alpha, 1. / self.beta);
        let precision = if precision == 0. || self.n == 0 { 0.001 } else { precision };
        let variance = 1. / precision;

        self.sampler.normal(self.mu, variance.sqrt())
    }

    /// Plays a game by taking action within given context. As result, updates a slot state.
    pub fn play(&self, context: A::Context) -> A::Feedback {
        self.action.take(context)
    }

    /// Updates slot machine.
    pub fn update(&mut self, feedback: &A::Feedback) {
        let reward = feedback.reward();

        let n = 1.;
        let v = self.n as Float;

        self.alpha += n / 2.;
        self.beta += (n * v / (v + n)) * (reward - self.mu).powi(2) / 2.;

        // estimate the variance: calculate running mean from the gamma hyper-parameters
        self.v = self.beta / (self.alpha + 1.);
        self.n += 1;
        self.mu += (reward - self.mu) / self.n as Float;
    }

    /// Gets learned params (alpha, beta, mean and variants) and usage amount.
    pub fn get_params(&self) -> (Float, Float, Float, Float, usize) {
        (self.alpha, self.beta, self.mu, self.v, self.n)
    }
}
