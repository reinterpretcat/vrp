#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/remedian_test.rs"]
mod remedian_test;

use std::cmp::Ordering;
use std::ops::ControlFlow;

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
    exponent: usize,
    buffers: Vec<Vec<T>>,
    count: usize,
    is_full: bool,
    order_fn: F,
}

impl<T, F> Remedian<T, F>
where
    T: Clone,
    F: Fn(&T, &T) -> Ordering,
{
    /// Creates a new instance of median estimator.
    /// `base`: the maximum size of a buffer (better to be odd). Recommended value: 11.
    /// `exponent`: the number of buffers. Max processed values is `base^exponent`.
    /// `order_fn`: ordering function.
    pub fn new(base: usize, exponent: usize, order_fn: F) -> Self {
        assert!(base > 0);

        let mut buffers: Vec<Vec<T>> = Vec::with_capacity(exponent);
        (0..exponent).for_each(|_| {
            buffers.push(Vec::with_capacity(base));
        });

        Self { base, exponent, buffers, count: 0, is_full: false, order_fn }
    }

    /// Adds a new observation.
    /// Returns true if the observation was added, false if the buffer is full.
    pub fn add_observation(&mut self, value: T) -> bool {
        if self.is_full {
            return false;
        }

        self.count += 1;
        self.buffers[0].push(value);

        let _ = (0..self.exponent).try_for_each(|i| {
            let batch = &mut self.buffers[i];

            if batch.len() == self.base {
                batch.sort_by(&self.order_fn);

                // not yet the last buffer, so calculate intermediate median and store it to the next buffer
                if i != self.exponent - 1 {
                    let median = batch[self.base / 2].clone();
                    batch.clear();

                    self.buffers[i + 1].push(median);
                } else {
                    self.is_full = true;
                }

                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        });

        true
    }

    /// Returns a median approximation if it is there.
    pub fn approx_median(&self) -> Option<T> {
        // buffers are full, return the last buffer's median
        if self.is_full {
            return Some(self.buffers[self.exponent - 1][self.base / 2].clone());
        }

        let mut weighted_medians = self
            .buffers
            .iter()
            .enumerate()
            .map(|(idx, buffer)| (buffer, (self.base as u64).pow(idx as u32)))
            .flat_map(|(buffer, w)| buffer.iter().map(move |m| (m, w)))
            .collect::<Vec<_>>();

        weighted_medians.sort_by(|(a, _), (b, _)| (self.order_fn)(a, b));

        let half_count = self.count as u64 / 2;
        weighted_medians
            .iter()
            .try_fold(0, |running_weight, (m, w)| {
                let running_weight = running_weight + w;
                if running_weight >= half_count {
                    return ControlFlow::Break(*m);
                }
                ControlFlow::Continue(running_weight)
            })
            .map_break(|m| m.clone())
            .break_value()
    }
}
