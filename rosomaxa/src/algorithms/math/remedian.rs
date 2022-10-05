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
/// Journal of the American Statistical  Association, vol. 85 (1990), pp. 97-104
/// - Domenico Cantone, Micha Hofri, "Further analysis of the remedian algorithm", Theoretical Computer
/// Science, vol. 495 (2013), pp. 1-16
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
        let _ = (0..).try_fold(value, |value, index| {
            if self.buffers.len() <= index {
                self.buffers.push(Vec::with_capacity(self.base))
            }
            let buffer = self.buffers.get_mut(index).unwrap();
            buffer.push(value);

            if buffer.len() < self.base {
                return Err(());
            }

            buffer.sort_by(&self.order_fn);
            let median_idx = self.base / 2;

            let value = buffer.get(median_idx).unwrap().clone();
            buffer.clear();

            Ok(value)
        });
    }

    /// Returns a median approximation if it is there.
    pub fn approx_median(&self) -> Option<T> {
        if self.buffers.is_empty() {
            None
        } else {
            self.buffers.last().and_then(|buffer| buffer.get(buffer.len() / 2)).cloned()
        }
    }
}
