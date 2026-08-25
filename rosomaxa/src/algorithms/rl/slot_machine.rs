#[cfg(test)]
#[path = "../../../tests/unit/algorithms/rl/slot_machine_test.rs"]
mod slot_machine_test;

use crate::utils::{DistributionSampler, Float};

const PRIOR_BETA: Float = 1.;
const MAX_EVIDENCE: Float = 100.;

/// Represents an action on slot machine.
pub trait SlotAction {
    /// An environment context.
    type Context;
    /// A feedback from taking slot action.
    type Feedback: SlotFeedback;

    /// Take an action for given context and return reward.
    fn take(&self, context: Self::Context) -> Self::Feedback;
}

/// Provides feedback for taking an action on a slot.
pub trait SlotFeedback {
    /// Returns whether the action produced the outcome learned by the slot machine.
    fn is_success(&self) -> bool;
}

/// Selects an action using non-stationary Thompson sampling with a Beta posterior.
///
/// The posterior models a binary outcome. Its evidence is capped to keep the selector responsive when
/// action effectiveness changes during the search.
#[derive(Clone)]
pub struct SlotMachine<A, S> {
    /// Alpha used to initialize and restart learning.
    prior_alpha: Float,
    /// The number of times this slot machine has been used (telemetry only).
    n: usize,
    /// Successful outcome evidence.
    alpha: Float,
    /// Unsuccessful outcome evidence.
    beta: Float,
    /// Sampler used to draw values from the posterior.
    sampler: S,
    /// The actual action associated with this slot.
    action: A,
}

impl<A, S> SlotMachine<A, S>
where
    A: SlotAction + Clone,
    S: DistributionSampler + Clone,
{
    /// Creates a new instance with the specified successful-outcome prior.
    pub fn new(prior_alpha: Float, action: A, sampler: S) -> Self {
        assert!(prior_alpha.is_finite() && prior_alpha > 0.);

        Self { prior_alpha, n: 0, alpha: prior_alpha, beta: PRIOR_BETA, sampler, action }
    }

    /// Samples the probability of success from the Beta posterior.
    pub fn sample(&self) -> Float {
        // A Beta sample can be obtained from two independent Gamma samples. Keeping the shapes positive
        // also protects very old posteriors whose unsuccessful evidence has driven alpha to underflow.
        let alpha = self.sampler.gamma(self.alpha.max(Float::EPSILON), 1.);
        let beta = self.sampler.gamma(self.beta.max(Float::EPSILON), 1.);
        let total = alpha + beta;

        if total.is_finite() && total > 0. { alpha / total } else { self.mean() }
    }

    /// Plays the slot machine by executing the action within the given context.
    pub fn play(&self, context: A::Context) -> A::Feedback {
        self.action.take(context)
    }

    /// Restores the initial posterior while preserving the lifetime usage counter.
    pub fn reset(&mut self) {
        self.alpha = self.prior_alpha;
        self.beta = PRIOR_BETA;
    }

    /// Updates the posterior and limits its confidence to the most recent effective evidence.
    pub fn update(&mut self, feedback: &A::Feedback) {
        let success = if feedback.is_success() { 1. } else { 0. };

        if self.alpha + self.beta >= MAX_EVIDENCE {
            // Dynamic Thompson Sampling discounts the posterior after reaching the evidence cap. This
            // bounds how much old evidence a changed success rate has to overcome.
            let scale = MAX_EVIDENCE / (self.alpha + self.beta + 1.);
            self.alpha = (self.alpha + success) * scale;
            self.beta = MAX_EVIDENCE - self.alpha;
        } else {
            self.alpha += success;
            self.beta += 1. - success;
        }

        self.n += 1;
    }

    /// Gets learned params (alpha, beta, mean, variance) and usage amount.
    pub fn get_params(&self) -> (Float, Float, Float, Float, usize) {
        let total = self.alpha + self.beta;
        let mean = self.mean();
        let variance = self.alpha * self.beta / (total.powi(2) * (total + 1.));

        (self.alpha, self.beta, mean, variance, self.n)
    }

    fn mean(&self) -> Float {
        self.alpha / (self.alpha + self.beta)
    }
}
