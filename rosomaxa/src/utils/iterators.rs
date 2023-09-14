//! This module provides some handy iterator extensions.

#[cfg(test)]
#[path = "../../tests/unit/utils/iterators_test.rs"]
mod iterators_test;

use crate::utils::*;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

/// An iterator which collects items into group.
pub trait CollectGroupBy: Iterator {
    /// Collects items into group.
    fn collect_group_by_key<K, V, FA>(self, f: FA) -> HashMap<K, Vec<V>>
    where
        Self: Sized + Iterator<Item = V>,
        K: Hash + Eq,
        FA: Fn(&V) -> K,
    {
        self.map(|v| (f(&v), v)).collect_group_by()
    }

    /// Collects items into group.
    fn collect_group_by<K, V>(self) -> HashMap<K, Vec<V>>
    where
        Self: Sized + Iterator<Item = (K, V)>,
        K: Hash + Eq,
    {
        let mut map = HashMap::new();

        for (key, val) in self {
            let vec: &mut Vec<_> = map.entry(key).or_default();
            vec.push(val);
        }

        map
    }
}

impl<T: Iterator> CollectGroupBy for T {}

/// An iterator which visits given range using selection sampling (Algorithm S).
pub struct SelectionSamplingIterator<I: Iterator> {
    processed: usize,
    needed: usize,
    size: usize,
    iterator: I,
    random: Arc<dyn Random + Send + Sync>,
}

impl<I: Iterator> SelectionSamplingIterator<I> {
    /// Creates a new instance of `SelectionSamplingIterator`.
    pub fn new(iterator: I, amount: usize, random: Arc<dyn Random + Send + Sync>) -> Self {
        assert!(amount > 0);
        Self {
            // NOTE relying on lower bound size hint!
            size: iterator.size_hint().0,
            processed: 0,
            needed: amount,
            iterator,
            random,
        }
    }
}

impl<I: Iterator> Iterator for SelectionSamplingIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let left = if self.needed != 0 && self.size > self.processed {
                self.size - self.processed
            } else {
                return None;
            };

            let probability = self.needed as f64 / left as f64;

            self.processed += 1;
            let next = self.iterator.next();

            if next.is_none() || self.random.is_hit(probability) {
                self.needed -= 1;
                return next;
            }
        }
    }
}

/// Returns a new iterator which samples some range from existing one.
pub fn create_range_sampling_iter<I: Iterator>(
    iterator: I,
    sample_size: usize,
    random: &(dyn Random + Send + Sync),
) -> impl Iterator<Item = I::Item> {
    let iterator_size = iterator.size_hint().0 as f64;
    let sample_count = (iterator_size / sample_size as f64).max(1.) - 1.;
    let offset = random.uniform_int(0, sample_count as i32) as usize * sample_size;

    iterator.skip(offset).take(sample_size)
}

/// Provides way to search with help of selection sampling algorithm on iterator where elements have
/// ordered index values.
///
/// The general idea is to sample values from the sequence uniformly, find the best from them and
/// check adjusted range, formed by these sampled values. The general motivation is that in many
/// domains values are not distributed randomly and this approach can quickly explore promising
/// regions and start exploiting them, significantly reducing total amount of probes.
///
/// For example:
///
/// - let's assume we have the following sequence: 48, 8, 45, 11, 21, 54, 15, 26, 23, 37, 58, 27, 31, 11, 60,
///   sampling size is 4 and we want to find a maximum value.
/// - at first iteration, let's assume it samples the following values from range [0, 14):
///     - 1 sample: 26 at 7
///     - 2 sample: 23 at 8
///     - 3 sample: 27 at 10
///     - 4 sample: 11 at 13
/// - the highest value is 27, so previous and next sampled indices (8, 13) give a next range to sample:
///     - 5 sample: 37 at 9
///     - 6 sample: 58 at 11
///     - 7 sample: 31 at 12
///  - here we found a better maximum (58), so we update current best and continue with further shrinking the search range
///  - we repeat the process till trivial range is reached
///
/// TODO: fixme: sometimes algorithm skips searching for a range (seems related to best as last element in the sequence)
///       see can_reproduce_issue_with_weak_sampling test
///       additionally, the code could be made a bit nicer and less hacky
pub trait SelectionSamplingSearch: Iterator {
    /// Searches using selection sampling algorithm.
    fn sample_search<'a, T, R, FM, FI, FC>(
        self,
        sample_size: usize,
        random: Arc<dyn Random + Send + Sync>,
        mut map_fn: FM,
        index_fn: FI,
        compare_fn: FC,
    ) -> Option<R>
    where
        Self: Sized + Clone + Iterator<Item = T> + 'a,
        T: 'a,
        R: 'a,
        FM: FnMut(T) -> R,
        FI: Fn(&T) -> i32,
        FC: Fn(&R, &R) -> bool,
    {
        let last_idx = i32::MAX;
        let mut state = SelectionSamplingSearchState::<R>::default();

        loop {
            let best_idx = state.best.as_ref().map_or(-1, |(best_idx, _)| *best_idx);
            let skip = state.target_left as usize;
            let take = (state.target_right - state.target_left) as usize + 1;

            state.next_left = last_idx;
            state.next_right = state.next_right.min(last_idx);

            state = SelectionSamplingIterator::new(self.clone().skip(skip).take(take), sample_size, random.clone())
                .filter_map(|item| {
                    let item_idx = index_fn(&item);
                    if item_idx != best_idx {
                        Some((item_idx, map_fn(item)))
                    } else {
                        None
                    }
                })
                .fold(state, |mut acc, (item_idx, item_mapped)| {
                    if acc.best.as_ref().map_or(false, |(best_idx, _)| *best_idx == acc.target_left) {
                        acc.next_right = item_idx - 1;
                    }

                    if acc.best.as_ref().map_or(true, |(_, best)| compare_fn(&item_mapped, best)) {
                        acc.best = Some((item_idx, item_mapped));
                        acc.next_left = acc.target_left + 1
                    }

                    acc.target_left = item_idx;

                    acc
                });

            state.target_left = state.next_left;
            state.target_right = state.next_right;

            if state.target_left >= state.target_right {
                break;
            }
        }

        state.best.map(|(_, best)| best)
    }
}

impl<T: Iterator> SelectionSamplingSearch for T {}

struct SelectionSamplingSearchState<T> {
    target_left: i32,
    target_right: i32,
    best: Option<(i32, T)>,
    next_left: i32,
    next_right: i32,
}

impl<T> Default for SelectionSamplingSearchState<T> {
    fn default() -> Self {
        Self { target_left: 0, target_right: i32::MAX, best: None, next_left: i32::MAX, next_right: i32::MAX }
    }
}

impl<T> std::fmt::Debug for SelectionSamplingSearchState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("target", &(self.target_left, self.target_right))
            .field("next", &(self.next_left, self.next_right))
            .field("best_idx", &self.best.as_ref().map_or("X".to_string(), |(best_idx, _)| best_idx.to_string()))
            .finish()
    }
}
