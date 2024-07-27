//! Specifies some logic to work with noise.

use crate::prelude::Random;
use std::sync::Arc;

/// Provides way to generate some noise to floating point value.
#[derive(Clone)]
pub struct Noise {
    probability: f64,
    range: (f64, f64),
    is_addition: bool,
    random: Arc<dyn Random>,
}

impl Noise {
    /// Creates a new instance of `Noise` which will add some noise in given range
    /// to the target value: `value = value + value * sample_from(range)`
    pub fn new_with_addition(probability: f64, range: (f64, f64), random: Arc<dyn Random>) -> Self {
        Self { probability, range, is_addition: true, random }
    }

    /// Creates a new instance of `Noise` which will apply noise by multiplying target value
    /// by value from given range: `value = value * sample_from(range)`
    pub fn new_with_ratio(probability: f64, range: (f64, f64), random: Arc<dyn Random>) -> Self {
        Self { probability, range, is_addition: false, random }
    }

    /// Generates an iterator with noise.
    pub fn generate_multi<'a, Iter: Iterator<Item = f64> + 'a>(
        &'a self,
        values: Iter,
    ) -> impl Iterator<Item = f64> + 'a {
        values.map(|value| value + self.generate(value))
    }

    /// Generate some noise based on given value.
    pub fn generate(&self, value: f64) -> f64 {
        if self.random.is_hit(self.probability) {
            // NOTE if value is zero, then noise is not applied which causes some troubles in edge cases
            if value == 0. {
                self.random.uniform_real(self.range.0, self.range.1)
            } else {
                value * self.random.uniform_real(self.range.0, self.range.1) + if self.is_addition { value } else { 0. }
            }
        } else {
            value
        }
    }

    /// Returns random generator.
    pub fn random(&self) -> &(dyn Random) {
        self.random.as_ref()
    }
}
