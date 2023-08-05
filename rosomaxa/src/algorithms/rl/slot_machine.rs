#[cfg(test)]
#[path = "../../../tests/unit/algorithms/rl/slot_machine_test.rs"]
mod slot_machine_test;

use crate::utils::DistributionSampler;
use std::fmt::{Display, Formatter};

/// Simulates a slot machine.
/// Internally tries to estimate reward probability distribution using one of methods from Thompson sampling.
#[derive(Clone)]
pub struct SlotMachine<T, S>
where
    T: Clone,
    S: Clone,
{
    /// The number of times this slot machine has been tried.
    n: usize,
    /// Gamma shape parameter.
    alpha: f64,
    /// Gamma rate parameter.
    beta: f64,
    /// Estimated mean.
    mu: f64,
    /// Estimated variance.
    v: f64,
    /// Sampler: used to provide samples from underlying estimated distribution.
    sampler: S,
    /// Actual slot play function.
    player: T,
}

impl<T, S> SlotMachine<T, S>
where
    T: Fn() -> f64 + Clone,
    S: DistributionSampler + Clone,
{
    /// Creates a new instance of `SlotMachine`.
    pub fn new(prior_mean: f64, sampler: S, player: T) -> Self {
        let alpha = 1.;
        let beta = 10.;
        let mu = prior_mean;
        let v = beta / (alpha + 1.);

        Self { n: 0, alpha, beta, mu, v, player, sampler }
    }

    /// Samples from estimated normal distribution.
    pub fn sample(&self) -> f64 {
        let precision = self.sampler.gamma(self.alpha, 1. / self.beta);
        let precision = if precision == 0. || self.n == 0 { 0.001 } else { precision };
        let variance = 1. / precision;

        self.sampler.normal(self.mu, variance.sqrt())
    }

    /// Plays the game and updates slot state.
    pub fn play(&mut self) {
        let reward = (self.player)();
        self.update(reward);
    }

    /// Updates slot machine.
    fn update(&mut self, reward: f64) {
        let n = 1.;
        let v = self.n as f64;

        self.alpha += n / 2.;
        self.beta += (n * v / (v + n)) * (reward - self.mu).powi(2) / 2.;

        // estimate the variance: calculate running mean from the gamma hyper-parameters
        self.v = self.beta / (self.alpha + 1.);
        self.n += 1;
        self.mu += (reward - self.mu) / self.n as f64;
    }
}

impl<T, S> Display for SlotMachine<T, S>
where
    T: Clone,
    S: Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "n={},alpha={},beta={},mu={},v={}", self.n, self.alpha, self.beta, self.mu, self.v)
    }
}
