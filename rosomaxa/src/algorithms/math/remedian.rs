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
    // Keeps buffer positions ordered by value, so reading the estimate does not need a temporary
    // allocation and sort. Positions from a compacted buffer are removed before it is reused.
    ordered: Vec<(usize, usize)>,
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

        Self {
            base,
            exponent,
            buffers,
            ordered: Vec::with_capacity(base.saturating_mul(exponent)),
            count: 0,
            is_full: false,
            order_fn,
        }
    }

    /// Adds a new observation.
    /// Returns true if the observation was added, false if the buffer is full.
    pub fn add_observation(&mut self, value: T) -> bool {
        if self.is_full {
            return false;
        }

        self.count += 1;
        self.buffers[0].push(value);
        self.insert_ordered(0);

        let _ = (0..self.exponent).try_for_each(|i| {
            if self.buffers[i].len() == self.base {
                // not yet the last buffer, so calculate intermediate median and store it to the next buffer
                if i != self.exponent - 1 {
                    let median = self
                        .ordered
                        .iter()
                        .filter(|(buffer_idx, _)| *buffer_idx == i)
                        .nth(self.base / 2)
                        .map(|(buffer_idx, value_idx)| self.buffers[*buffer_idx][*value_idx].clone())
                        .expect("cannot get intermediate median");

                    self.ordered.retain(|(buffer_idx, _)| *buffer_idx != i);
                    self.buffers[i].clear();

                    self.buffers[i + 1].push(median);
                    self.insert_ordered(i + 1);
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
        let half_count = self.count as u64 / 2;
        self.ordered
            .iter()
            .try_fold(0, |running_weight, (buffer_idx, value_idx)| {
                let running_weight = running_weight + (self.base as u64).pow(*buffer_idx as u32);
                if running_weight >= half_count {
                    return ControlFlow::Break(&self.buffers[*buffer_idx][*value_idx]);
                }
                ControlFlow::Continue(running_weight)
            })
            .map_break(|m| m.clone())
            .break_value()
    }

    fn insert_ordered(&mut self, buffer_idx: usize) {
        let value_idx = self.buffers[buffer_idx].len() - 1;
        let value = &self.buffers[buffer_idx][value_idx];
        let insert_idx = self.ordered.partition_point(|(other_buffer_idx, other_value_idx)| {
            (self.order_fn)(&self.buffers[*other_buffer_idx][*other_value_idx], value) != Ordering::Greater
        });

        self.ordered.insert(insert_idx, (buffer_idx, value_idx));
    }
}
