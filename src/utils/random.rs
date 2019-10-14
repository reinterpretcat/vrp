extern crate rand;

use self::rand::Rng;

/// Provides the way to use randomized values in generic way.
pub trait Random {
    /// Produces integral random value, uniformly distributed on the closed interval [min, max)
    fn uniform_int(&self, min: i32, max: i32) -> i32 {
        rand::thread_rng().gen_range(min, max)
    }

    /// Produces real random value, uniformly distributed on the closed interval [min, max)
    fn uniform_real(&self, min: f64, max: f64) -> f64 {
        rand::thread_rng().gen_range(min, max)
    }

    /// Flips a coin and returns true if it is "heads", false otherwise.
    fn is_head_not_tails(&self) -> bool {
        self.uniform_int(0, 2) > 0
    }
}
