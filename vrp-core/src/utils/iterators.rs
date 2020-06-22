//! This module provides some handy iterator extensions.

use hashbrown::HashMap;
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
            let vec = map.entry(key).or_insert(Vec::new());
            vec.push(val);
        }

        map
    }
}

impl<T: Iterator> CollectGroupBy for T {}
