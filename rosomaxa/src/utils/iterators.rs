//! This module provides some handy iterator extensions.

#[cfg(test)]
#[path = "../../tests/unit/utils/iterators_test.rs"]
mod iterators_test;

use crate::utils::Random;
use hashbrown::HashMap;
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

    iterator.skip(offset).take(sample_size as usize)
}

/// Provides way to search using selection sampling algorithm on iterator where elements have ordered
/// index values.
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
                .filter(|item| index_fn(item) != best_idx)
                .fold(state, |mut acc, item| {
                    let item_idx = index_fn(&item);
                    let item_mapped = map_fn(item);

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

#[derive(Debug)]
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
