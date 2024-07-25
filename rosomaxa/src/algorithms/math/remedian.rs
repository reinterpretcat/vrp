#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/remedian_test.rs"]
mod remedian_test;

use std::cmp::Ordering;

/// Specifies a median estimator used to track medians of heuristic running time.
pub type RemedianUsize = Remedian<usize, fn(&usize, &usize) -> Ordering>;

/// A custom implementation of robust median estimator.
///
/// References:
/// - P.J. Rousseeuw, G.W. Bassett Jr., "The remedian: A robust averaging method for large data sets",
///   Journal of the American Statistical  Association, vol. 85 (1990), pp. 97-104
/// - Domenico Cantone, Micha Hofri, "Further analysis of the remedian algorithm", Theoretical Computer
///   Science, vol. 495 (2013), pp. 1-16
pub struct Remedian<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> Ordering,
{
    base: usize,
    buffers: Vec<Vec<T>>,
    order_fn: F,
}

impl<T, F> Remedian<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> Ordering,
{
    /// Creates a new instance of median estimator.
    /// `base`: the maximum size of a buffer (better to be odd)
    /// `order_fn`: ordering function.
    pub fn new(base: usize, order_fn: F) -> Self {
        assert!(base > 0);

        Self { base, buffers: vec![], order_fn }
    }

    /// Adds a new observation.
    pub fn add_observation(&mut self, value: T) {
        let _ = (0..).try_fold(value, |value, idx| {
            if self.buffers.len() <= idx {
                self.buffers.push(Vec::with_capacity(self.base))
            }

            let buffer = self.buffers.get_mut(idx).unwrap();
            buffer.push(value);

            if buffer.len() < self.base {
                return Err(());
            }

            buffer.sort_by(&self.order_fn);

            let value = buffer.get(self.base / 2).unwrap().clone();
            buffer.clear();

            // NOTE: use only two buffers, buffer at index 0 should be already clean
            if idx == 1 {
                buffer.push(value);
                debug_assert!(self.buffers[0].is_empty());
                Err(())
            } else {
                Ok(value)
            }
        });
    }

    /// Returns a median approximation if it is there.
    pub fn approx_median(&self) -> Option<T> {
        let has_not_enough_observations = self.buffers.len() == 1 && self.buffers[0].len() < self.base;
        if self.buffers.is_empty() || has_not_enough_observations {
            None
        } else {
            self.buffers.last().and_then(|buffer| buffer.last()).cloned()
        }
    }
}
