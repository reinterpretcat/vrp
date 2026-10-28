#[cfg(test)]
#[path = "../../../tests/unit/algorithms/rl/slot_machine_test.rs"]
mod slot_machine_test;

use crate::utils::{DistributionSampler, Float};

const PRIOR_BETA: Float = 1.;
const MAX_EVIDENCE: Float = 100.;

/// State of a Beta posterior over a Bernoulli outcome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BernoulliParams {
    pub alpha: Float,
    pub beta: Float,
    pub mean: Float,
    pub variance: Float,
    pub observations: usize,
}

/// A capped Beta posterior used to learn a non-stationary Bernoulli outcome.
#[derive(Clone)]
pub(crate) struct BernoulliPosterior<S> {
    prior_alpha: Float,
    alpha: Float,
    beta: Float,
    observations: usize,
    sampler: S,
}

impl<S> BernoulliPosterior<S>
where
    S: DistributionSampler + Clone,
{
    pub fn new(prior_alpha: Float, sampler: S) -> Self {
        assert!(prior_alpha.is_finite() && prior_alpha > 0.);

        Self { prior_alpha, alpha: prior_alpha, beta: PRIOR_BETA, observations: 0, sampler }
    }

    /// Samples the probability of success from the posterior.
    pub fn sample(&self) -> Float {
        // A Beta sample can be obtained from two independent Gamma samples. Keeping the shapes positive
        // also protects very old posteriors whose unsuccessful evidence has driven alpha to underflow.
        let alpha = self.sampler.gamma(self.alpha.max(Float::EPSILON), 1.);
        let beta = self.sampler.gamma(self.beta.max(Float::EPSILON), 1.);
        let total = alpha + beta;

        if total.is_finite() && total > 0. { alpha / total } else { self.params().mean }
    }

    /// Restores the initial posterior.
    pub fn reset(&mut self) {
        self.alpha = self.prior_alpha;
        self.beta = PRIOR_BETA;
    }

    /// Updates the posterior and limits its confidence to the most recent effective evidence.
    pub fn update(&mut self, is_success: bool) {
        let success = if is_success { 1. } else { 0. };

        if self.alpha + self.beta >= MAX_EVIDENCE {
            // Discount the posterior after reaching the evidence cap. This bounds how much old evidence
            // a changed success rate has to overcome.
            let scale = MAX_EVIDENCE / (self.alpha + self.beta + 1.);
            self.alpha = (self.alpha + success) * scale;
            self.beta = MAX_EVIDENCE - self.alpha;
        } else {
            self.alpha += success;
            self.beta += 1. - success;
        }

        self.observations = self.observations.saturating_add(1);
    }

    /// Returns the current posterior parameters.
    pub fn params(&self) -> BernoulliParams {
        let total = self.alpha + self.beta;
        let mean = self.alpha / total;
        let variance = self.alpha * self.beta / (total.powi(2) * (total + 1.));

        BernoulliParams { alpha: self.alpha, beta: self.beta, mean, variance, observations: self.observations }
    }
}

/// Represents an action on slot machine.
pub trait SlotAction {
    /// An environment context.
    type Context;
    /// A feedback from taking slot action.
    type Feedback: SlotFeedback;

    /// Takes an action for the given context and returns its feedback.
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
    /// Learned outcome distribution.
    posterior: BernoulliPosterior<S>,
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
        Self { posterior: BernoulliPosterior::new(prior_alpha, sampler), action }
    }

    /// Samples the probability of success from the Beta posterior.
    pub fn sample(&self) -> Float {
        self.posterior.sample()
    }

    /// Plays the slot machine by executing the action within the given context.
    pub fn play(&self, context: A::Context) -> A::Feedback {
        self.action.take(context)
    }

    /// Restores the initial posterior while preserving the lifetime usage counter.
    pub fn reset(&mut self) {
        self.posterior.reset();
    }

    /// Updates the posterior and limits its confidence to the most recent effective evidence.
    pub fn update(&mut self, feedback: &A::Feedback) {
        self.posterior.update(feedback.is_success());
    }

    /// Gets learned posterior parameters and lifetime usage.
    pub(crate) fn get_params(&self) -> BernoulliParams {
        self.posterior.params()
    }
}
