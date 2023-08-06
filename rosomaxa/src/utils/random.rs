#[cfg(test)]
#[path = "../../tests/unit/utils/random_test.rs"]
mod random_test;

use rand::prelude::*;
use rand::Error;
use rand_distr::{Gamma, Normal};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::sync::Arc;

/// Provides the way to sample from different distributions.
pub trait DistributionSampler {
    /// Returns a sample from gamma distribution.
    fn gamma(&self, shape: f64, scale: f64) -> f64;

    /// Returns a sample from normal distribution.
    fn normal(&self, mean: f64, std_dev: f64) -> f64;
}

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

/// Provides way to sample from different distributions.
#[derive(Clone)]
pub struct DefaultDistributionSampler(Arc<dyn Random + Send + Sync>);

impl DefaultDistributionSampler {
    /// Creates a new instance of `DefaultDistributionSampler`.
    pub fn new(random: Arc<dyn Random + Send + Sync>) -> Self {
        Self(random)
    }
}

impl DistributionSampler for DefaultDistributionSampler {
    fn gamma(&self, shape: f64, scale: f64) -> f64 {
        Gamma::new(shape, scale)
            .unwrap_or_else(|_| panic!("cannot create gamma dist: shape={shape}, scale={scale}"))
            .sample(&mut self.0.get_rng())
    }

    fn normal(&self, mean: f64, std_dev: f64) -> f64 {
        Normal::new(mean, std_dev)
            .unwrap_or_else(|_| panic!("cannot create normal dist: mean={mean}, std_dev={std_dev}"))
            .sample(&mut self.0.get_rng())
    }
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
        RandomGen::new_randomized()
    }
}

thread_local! {
    /// Random generator seeded from thread_rng to make runs non-repeatable.
    static RANDOMIZED_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_rng(thread_rng()).expect("cannot get RNG from thread rng"));

    /// Random generator seeded with 0 SmallRng to make runs repeatable.
    static REPEATABLE_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(0));
}

/// Provides underlying random generator API.
#[derive(Clone, Debug)]
pub struct RandomGen {
    use_repeatable: bool,
}

impl RandomGen {
    /// Creates an instance of `RandomGen` using random generator with fixed seed.
    pub fn new_repeatable() -> Self {
        Self { use_repeatable: true }
    }

    /// Creates an instance of `RandomGen` using random generator with randomized seed.
    pub fn new_randomized() -> Self {
        Self { use_repeatable: false }
    }
}

impl RngCore for RandomGen {
    fn next_u32(&mut self) -> u32 {
        // NOTE use 'likely!' macro for better branch prediction once it is stabilized?
        if self.use_repeatable {
            REPEATABLE_RNG.with(|t| t.borrow_mut().next_u32())
        } else {
            RANDOMIZED_RNG.with(|t| t.borrow_mut().next_u32())
        }
    }

    fn next_u64(&mut self) -> u64 {
        if self.use_repeatable {
            REPEATABLE_RNG.with(|t| t.borrow_mut().next_u64())
        } else {
            RANDOMIZED_RNG.with(|t| t.borrow_mut().next_u64())
        }
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        if self.use_repeatable {
            REPEATABLE_RNG.with(|t| t.borrow_mut().fill_bytes(dest))
        } else {
            RANDOMIZED_RNG.with(|t| t.borrow_mut().fill_bytes(dest))
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        if self.use_repeatable {
            REPEATABLE_RNG.with(|t| t.borrow_mut().try_fill_bytes(dest))
        } else {
            RANDOMIZED_RNG.with(|t| t.borrow_mut().try_fill_bytes(dest))
        }
    }
}

impl CryptoRng for RandomGen {}

/// Returns an index of max element in values. In case of many same max elements,
/// returns the one from them at random.
pub fn random_argmax<I>(values: I, random: &dyn Random) -> Option<usize>
where
    I: Iterator<Item = f64>,
{
    let mut rng = random.get_rng();
    let mut count = 0;
    values
        .enumerate()
        .max_by(move |(_, r), (_, s)| match r.total_cmp(s) {
            Ordering::Equal => {
                count += 1;
                if rng.gen_range(0..=count) == 0 {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            Ordering::Less => {
                count = 0;
                Ordering::Less
            }
            Ordering::Greater => Ordering::Greater,
        })
        .map(|(idx, _)| idx)
}
