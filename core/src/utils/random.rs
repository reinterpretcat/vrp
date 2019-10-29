extern crate rand;

use self::rand::Rng;
use std::slice::Iter;

/// Provides the way to use randomized values in generic way.
pub trait Random {
    /// Produces integral random value, uniformly distributed on the closed interval [min, max]
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        if min == max {
            return min;
        }

        assert!(min < max);
        rand::thread_rng().gen_range(min, max + 1)
    }

    /// Produces real random value, uniformly distributed on the closed interval [min, max)
    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        assert!(min < max);
        rand::thread_rng().gen_range(min, max)
    }

    /// Flips a coin and returns true if it is "heads", false otherwise.
    fn is_head_not_tails(&self) -> bool {
        self.uniform_int(1, 2) == 1
    }

    /// Returns an index from collected with probability weight.
    /// Uses exponential distribution where the weights are the rate of the distribution (lambda)
    /// and selects the smallest sampled value.
    fn weighted(&self, weights: Iter<usize>) -> usize {
        weights
            .zip(0usize..)
            .map(|(&weight, index)| (-self.uniform_real(0., 1.).ln() / weight as f64, index))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .1
    }
}

pub struct DefaultRandom {}

impl Random for DefaultRandom {}

impl DefaultRandom {
    pub fn new() -> Self {
        Self {}
    }
}
