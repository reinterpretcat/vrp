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
            let vec = map.entry(key).or_insert(Vec::new());
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
