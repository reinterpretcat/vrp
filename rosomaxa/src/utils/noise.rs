//! Specifies some logic to work with noise.

use crate::prelude::Random;
use std::sync::Arc;

/// Provides way to generate some noise to floating point value.
#[derive(Clone)]
pub struct Noise {
    probability: f64,
    range: (f64, f64),
    random: Arc<dyn Random + Send + Sync>,
}

impl Noise {
    /// Creates a new instance of `Noise`.
    pub fn new(probability: f64, range: (f64, f64), random: Arc<dyn Random + Send + Sync>) -> Self {
        Self { probability, range, random }
    }

    /// Adds some noise to given value.
    pub fn add(&self, value: f64) -> f64 {
        if self.random.is_hit(self.probability) {
            value * self.random.uniform_real(self.range.0, self.range.1)
        } else {
            value
        }
    }
}
