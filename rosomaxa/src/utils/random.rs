#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod random_test;

use rand::prelude::*;
use rand::Error;
use std::cell::UnsafeCell;
use std::rc::Rc;

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
    fn get_rng(&self) -> RandomGen;
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

    fn get_rng(&self) -> RandomGen {
        let rng = DEFAULT_RNG.with(|t| t.clone());
        RandomGen { rng }
    }
}

thread_local! {
    static DEFAULT_RNG: Rc<UnsafeCell<SmallRng>> = Rc::new(UnsafeCell::new(SmallRng::from_rng(thread_rng()).expect("cannot get RNG")));
}

/// Specifies underlying random generator type.
#[derive(Clone, Debug)]
pub struct RandomGen {
    rng: Rc<UnsafeCell<SmallRng>>,
}

impl RandomGen {
    /// Creates a new instance of `RandomGen` using given reference to small rng.
    pub fn with_rng(rng: Rc<UnsafeCell<SmallRng>>) -> Self {
        Self { rng }
    }
}

impl RngCore for RandomGen {
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        let rng = unsafe { &mut *self.rng.get() };
        rng.next_u32()
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let rng = unsafe { &mut *self.rng.get() };
        rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let rng = unsafe { &mut *self.rng.get() };
        rng.fill_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        let rng = unsafe { &mut *self.rng.get() };
        rng.try_fill_bytes(dest)
    }
}

impl CryptoRng for RandomGen {}
