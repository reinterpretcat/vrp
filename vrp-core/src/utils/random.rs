#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod random_test;

use rand::prelude::*;

/// Provides the way to use randomized values in generic way.
pub trait Random {
    /// Produces integral random value, uniformly distributed on the closed interval [min, max]
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        if min == max {
            return min;
        }

        assert!(min < max);
        self.get_rng().gen_range(min, max + 1)
    }

    /// Produces real random value, uniformly distributed on the closed interval [min, max)
    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        if (min - max).abs() < std::f64::EPSILON {
            return min;
        }

        assert!(min < max);
        self.get_rng().gen_range(min, max)
    }

    /// Flips a coin and returns true if it is "heads", false otherwise.
    fn is_head_not_tails(&self) -> bool {
        self.uniform_int(1, 2) == 1
    }

    /// Returns an index from collected with probability weight.
    /// Uses exponential distribution where the weights are the rate of the distribution (lambda)
    /// and selects the smallest sampled value.
    fn weighted(&self, weights: &[usize]) -> usize {
        weights
            .iter()
            .zip(0_usize..)
            .map(|(&weight, index)| (-self.uniform_real(0., 1.).ln() / weight as f64, index))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .1
    }

    /// Returns RNG.
    fn get_rng(&self) -> StdRng;
}

/// A default random implementation.
pub struct DefaultRandom {
    seed: Option<u64>,
}

impl DefaultRandom {
    /// Creates a new instance `DefaultRandom` with seed.
    pub fn new_with_seed(seed: u64) -> Self {
        Self { seed: Some(seed) }
    }
}

impl Random for DefaultRandom {
    fn get_rng(&self) -> StdRng {
        if let Some(ref seed) = self.seed {
            StdRng::seed_from_u64(*seed)
        } else {
            StdRng::from_rng(thread_rng()).expect("cannot get RNG")
        }
    }
}

impl Default for DefaultRandom {
    fn default() -> Self {
        Self { seed: None }
    }
}
