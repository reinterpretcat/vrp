#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod random_test;

use rand::prelude::*;
use std::cell::UnsafeCell;

/// Specifies underlying random generator type.
pub type RandomGen = SmallRng;

/// Provides the way to use randomized values in generic way.
pub trait Random {
    /// Produces integral random value, uniformly distributed on the closed interval [min, max]
    fn uniform_int(&self, min: i32, max: i32) -> i32;

    /// Produces real random value, uniformly distributed on the closed interval [min, max)
    fn uniform_real(&self, min: f64, max: f64) -> f64;

    /// Flips a coin and returns true if it is "heads", false otherwise.
    fn is_head_not_tails(&self) -> bool;

    /// Tests probability value in (0., 1.) range.
    fn is_hit(&self, probability: f64) -> bool;

    /// Returns an index from collected with probability weight.
    /// Uses exponential distribution where the weights are the rate of the distribution (lambda)
    /// and selects the smallest sampled value.
    fn weighted(&self, weights: &[usize]) -> usize;

    /// Returns RNG.
    /// NOTE: it returns a mutual reference on purpose to allow some performance optimizations.
    #[allow(clippy::mut_from_ref)]
    fn get_rng(&self) -> &mut RandomGen;
}

thread_local! {
    static DEFAULT_RNG: UnsafeCell<RandomGen> = UnsafeCell::new(SmallRng::from_rng(thread_rng()).expect("cannot get RNG"));
}

/// A default random implementation.
#[derive(Default)]
pub struct DefaultRandom {}

impl Random for DefaultRandom {
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        if min == max {
            return min;
        }

        assert!(min < max);
        self.get_rng().gen_range(min..max + 1)
    }

    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        if (min - max).abs() < f64::EPSILON {
            return min;
        }

        assert!(min < max);
        self.get_rng().gen_range(min..max)
    }

    fn is_head_not_tails(&self) -> bool {
        self.get_rng().gen_bool(0.5)
    }

    fn is_hit(&self, probability: f64) -> bool {
        self.get_rng().gen_bool(probability.clamp(0., 1.))
    }

    fn weighted(&self, weights: &[usize]) -> usize {
        weights
            .iter()
            .zip(0_usize..)
            .map(|(&weight, index)| (-self.uniform_real(0., 1.).ln() / weight as f64, index))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .1
    }

    fn get_rng(&self) -> &mut RandomGen {
        unsafe { &mut *DEFAULT_RNG.with(|cell| cell.get()) }
    }
}
