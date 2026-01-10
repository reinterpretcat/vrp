#[cfg(test)]
#[path = "../../../tests/unit/algorithms/rl/slot_machine_test.rs"]
mod slot_machine_test;

use crate::utils::{DistributionSampler, Float};

/// Represents an action on slot machine.
pub trait SlotAction {
    /// An environment context.
    type Context;
    /// A feedback from taking slot action.
    type Feedback: SlotFeedback;

    /// Take an action for given context and return reward.
    fn take(&self, context: Self::Context) -> Self::Feedback;
}

/// Provides a feedback for taking an action on a slot.
pub trait SlotFeedback {
    /// A reward for taking an action on a slot machine.
    fn reward(&self) -> Float;
}

/// Simulates a slot machine using Non-Stationary Thompson Sampling.
///
/// This implementation uses a Normal-Inverse-Gamma (NIG) conjugate prior to model
/// the unknown mean and variance of the reward distribution. It employs exponential
/// decay (weighted likelihood) to handle non-stationary environments where the
/// effectiveness of operators changes over time (e.g., VRP search phases).
#[derive(Clone)]
pub struct SlotMachine<A, S> {
    /// The number of times this slot machine has been used (telemetry only).
    n: usize,
    /// Shape parameter (α) of the Inverse-Gamma distribution (tracks sample count/confidence).
    alpha: Float,
    /// Rate parameter (β) of the Inverse-Gamma distribution (tracks sum of squared errors).
    beta: Float,
    /// Estimated mean (μ) of the Normal distribution.
    mu: Float,
    /// Estimated variance (E[σ²]) derived from α and β.
    v: Float,
    /// Sampler used to draw values from the estimated distribution.
    sampler: S,
    /// The actual action associated with this slot.
    action: A,
}

impl<A, S> SlotMachine<A, S>
where
    A: SlotAction + Clone,
    S: DistributionSampler + Clone,
{
    /// Creates a new instance with Universal Priors.
    ///
    /// Since the `SearchAgent` externally normalizes rewards to standard scores (Z-scores),
    /// we can use universal constants for the priors rather than problem-specific guesses.
    pub fn new(prior_mean: Float, action: A, sampler: S) -> Self {
        // Universal priors for a Standard Normal distribution N(0, 1):
        // Alpha = 2.0 implies a weak prior belief with mathematically defined variance.
        // Beta = 1.0 combined with Alpha=2.0 implies an expected variance of ~1.0.
        let alpha = 2.0;
        let beta = 1.0;

        // Prior mean is clamped to a reasonable Z-score range (-1 to 1) to prevent
        // initialization bias, though typically 0.0 is used for centered data.
        let mu = prior_mean.min(1.0).max(-1.0);

        // Variance expectation for Inverse-Gamma: Beta / (Alpha - 1)
        // With Alpha=2.0, this results in v=1.0.
        let v = beta / (alpha - 1.0);

        Self { n: 0, alpha, beta, mu, v, action, sampler }
    }

    /// Samples a reward prediction from the estimated Normal-Inverse-Gamma distribution.
    ///
    /// 1. Samples precision (τ) from Gamma(α, β).
    /// 2. Samples reward from Normal(μ, 1/√(τ)).
    pub fn sample(&self) -> Float {
        // Sample precision from Gamma distribution
        let precision = self.sampler.gamma(self.alpha, 1. / self.beta);

        // Safety: If precision is numerically zero (rare), fallback to high variance
        let precision = if precision == 0. || self.n == 0 { 0.001 } else { precision };
        let variance = 1. / precision;

        self.sampler.normal(self.mu, variance.sqrt())
    }

    /// Plays the slot machine by executing the action within the given context.
    pub fn play(&self, context: A::Context) -> A::Feedback {
        self.action.take(context)
    }

    /// Updates the internal Bayesian state with a new reward observation.
    ///
    /// The update logic performs two key functions:
    /// 1. **Decay:** Forgets old observations to adapt to the changing search landscape.
    /// 2. **Bayesian Update:** Refines estimates of Mean and Variance using the new data.
    ///
    /// `reward` is expected to be a normalized relative value (e.g., success ≈ 1.0, failure = 0.0).
    pub fn update(&mut self, feedback: &A::Feedback) {
        let reward = feedback.reward();

        // --- 1. Memory Decay (Non-Stationarity) ---

        // A decay factor of 0.999 implies a "memory horizon" of ~1000 samples.
        // This is crucial for VRP where improvements are rare (sparse signals).
        // It provides enough patience to wait for "lottery ticket" wins while still
        // allowing the agent to abandon operators that stop working in later phases.
        const DECAY_FACTOR: Float = 0.999;

        // Decay the sufficient statistics.
        // CRITICAL: We clamp alpha to >= 2.0. The variance of the Inverse-Gamma
        // distribution is defined as Beta / (Alpha - 1). If Alpha <= 1, variance is undefined.
        // Keeping Alpha >= 2.0 ensures numerical stability and prevents division by zero.
        self.alpha = (self.alpha * DECAY_FACTOR).max(2.0);
        self.beta *= DECAY_FACTOR;

        // Increment usage counter (purely for human telemetry/diagnostics).
        // We do not decay this value so we can track total lifetime usage.
        self.n += 1;

        // --- 2. Bayesian Update (Normal-Gamma) ---

        // Standard update adds 0.5 to Alpha for each new observation n=1.
        self.alpha += 0.5;
        let old_mu = self.mu;

        // Calculate Effective N derived from shape parameter.
        // In Normal-Gamma, Alpha grows by 0.5 per sample, so N ~ 2 * Alpha.
        // This avoids maintaining a separate floating-point 'n' variable for the math.
        let effective_n = self.alpha * 2.0;

        // Update Mean (Mu)
        // Uses linear interpolation based on the effective sample size.
        let learning_rate = 1.0 / effective_n;
        self.mu += learning_rate * (reward - self.mu);

        // Update Variance (Beta)
        // This is the Bayesian adaptation of Welford's online variance algorithm.
        // It incrementally updates the sum of squared errors.
        // The term `effective_n / (effective_n + 1.0)` weights the new sample's
        // contribution to the variance relative to prior knowledge.
        self.beta += 0.5 * (reward - old_mu).powi(2) * effective_n / (effective_n + 1.0);

        // --- 3. Variance Estimation ---

        // Calculate expected variance E[σ²] = Beta / (Alpha - 1).
        // Since we enforced Alpha >= 2.0 (decayed) + 0.5 (update) = 2.5,
        // the denominator is guaranteed to be >= 1.5. Safe division.
        self.v = self.beta / (self.alpha - 1.0);
    }

    /// Gets learned params (alpha, beta, mean, variance) and usage amount.
    pub fn get_params(&self) -> (Float, Float, Float, Float, usize) {
        (self.alpha, self.beta, self.mu, self.v, self.n)
    }
}
