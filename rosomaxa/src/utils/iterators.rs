//! This module provides some handy iterator extensions.

#[cfg(test)]
#[path = "../../tests/unit/utils/iterators_test.rs"]
mod iterators_test;

use crate::utils::*;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

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
pub struct SelectionSamplingIterator<I: Iterator, R: Random> {
    processed: usize,
    needed: usize,
    size: usize,
    iterator: I,
    random: R,
}

impl<I: Iterator, R: Random> SelectionSamplingIterator<I, R> {
    /// Creates a new instance of `SelectionSamplingIterator`.
    pub fn new(iterator: I, amount: usize, random: R) -> Self {
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

impl<I: Iterator, R: Random> Iterator for SelectionSamplingIterator<I, R> {
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
pub fn create_range_sampling_iter<I: Iterator, R: Random>(
    iterator: I,
    sample_size: usize,
    random: R,
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
pub trait SelectionSamplingSearch: Iterator {
    /// Searches using selection sampling algorithm.
    fn sample_search<'a, T, R, FM, FI, FC, RNG>(
        self,
        sample_size: usize,
        random: RNG,
        mut map_fn: FM,
        index_fn: FI,
        compare_fn: FC,
    ) -> Option<R>
    where
        Self: Sized + Clone + Iterator<Item = T> + 'a,
        T: 'a,
        R: 'a,
        FM: FnMut(T) -> R,
        FI: Fn(&T) -> usize,
        FC: Fn(&R, &R) -> bool,
        RNG: Random,
    {
        // support up to 32*8 indices to be memorized
        const N: usize = 32;

        let size = self.size_hint().0;
        if size == 0 || sample_size == 0 {
            return None;
        }

        let mut state = SearchState::<N, R>::new(sample_size, size);
        loop {
            let (skip, take) = (state.left, state.right - state.left + 1);
            let iterator = self.clone().skip(skip).take(take);
            // keeps track data to track properly right range limit if best is found at last
            let (orig_right, last_probe_idx) = (state.right, take.min(sample_size - 1));

            state = SelectionSamplingIterator::new(iterator, sample_size, random.clone())
                .enumerate()
                .fold(state, |mut acc, (probe_idx, item)| {
                    let item_idx = index_fn(&item);
                    let is_new_item = acc.probe(item_idx);

                    assert!(
                        item_idx >= skip && item_idx <= orig_right,
                        "caller's index_fn returns an index outside of expected range"
                    );

                    // NOTE below we apply minus/plus one to border indices to avoid probing them multiple times
                    match &acc.best {
                        BestItem::Unknown => acc.best = BestItem::Fresh((item_idx, map_fn(item))),
                        BestItem::Fresh((best_idx, best_value)) | BestItem::Stale((best_idx, best_value)) => {
                            // if stale, shrink the range to converge the search
                            if matches!(acc.best, BestItem::Stale(_)) {
                                acc.left = ((item_idx + 1).min(*best_idx)).max(acc.left);
                                acc.right = ((item_idx.max(1) - 1).max(*best_idx)).min(acc.right);
                            } else {
                                //  if a new best is found on the previous probe, adjust right to the current probe
                                if acc.last == *best_idx {
                                    acc.right = item_idx.max(1) - 1
                                }
                            }

                            // avoid evaluating same item twice by checking the probe
                            if is_new_item {
                                let item_value = map_fn(item);
                                // if a new found, set the search range to adjusted left and right items
                                if compare_fn(&item_value, best_value) {
                                    acc.best = BestItem::Fresh((item_idx, item_value));
                                    // keep same index for left/right if it is a first/last probe
                                    acc.left = if probe_idx == 0 { acc.left } else { acc.last + 1 };
                                    acc.right = if probe_idx == last_probe_idx { orig_right } else { item_idx };
                                }
                            }
                        }
                    }

                    acc.last = item_idx;

                    acc
                })
                .next_range();

            if state.is_terminal() {
                break;
            }
        }

        state.best.get_value()
    }
}

impl<T: Iterator> SelectionSamplingSearch for T {}

/// Keeps track of best item index and actual value.
enum BestItem<T> {
    /// No best item yet discovered.
    Unknown,
    /// A best item was discovered, but on previous range search.
    Stale((usize, T)),
    /// A best item was discovered on current range search.
    Fresh((usize, T)),
}

impl<T> BestItem<T> {
    /// Gets value of best item if it is found.
    fn get_value(self) -> Option<T> {
        match self {
            BestItem::Unknown => None,
            BestItem::Stale((_, value)) | BestItem::Fresh((_, value)) => Some(value),
        }
    }
}

impl<T> Debug for BestItem<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BestItem::Unknown => write!(f, "X"),
            BestItem::Stale((idx, _)) | BestItem::Fresh((idx, _)) => write!(f, "{idx}"),
        }
    }
}

/// Keeps  track of search state for selection sampling search.
struct SearchState<const N: usize, T> {
    left: usize,
    right: usize,
    last: usize,
    best: BestItem<T>,
    bit_array: FixedBitArray<N>,
    collisions_limit: i32,
}

impl<const N: usize, T> SearchState<N, T> {
    pub fn new(collisions_limit: usize, size: usize) -> Self {
        Self {
            left: 0,
            right: size - 1,
            last: 0,
            best: BestItem::<T>::Unknown,
            bit_array: FixedBitArray::<N>::default(),
            collisions_limit: collisions_limit as i32,
        }
    }

    /// Returns true if item was not seen before.
    pub fn probe(&mut self, index: usize) -> bool {
        if self.bit_array.replace(index, true) {
            self.collisions_limit -= 1;
            false
        } else {
            true
        }
    }

    pub fn next_range(self) -> Self {
        Self {
            best: match self.best {
                BestItem::Unknown => BestItem::Unknown,
                BestItem::Stale(item) | BestItem::Fresh(item) => BestItem::Stale(item),
            },
            ..self
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.left >= self.right || self.collisions_limit <= 0
    }
}

impl<const N: usize, T> Debug for SearchState<N, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("range", &(self.left, self.right))
            .field("col_lim", &self.collisions_limit)
            .field("best_idx", &self.best)
            .field("bits", &format!("{:b}", self.bit_array))
            .finish()
    }
}
