use crate::json::Location;
use std::cmp::Ordering::Less;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Represents coordinate index.
pub struct CoordIndex {
    direct_index: HashMap<Location, usize>,
    reverse_index: HashMap<usize, Location>,
}

impl Default for CoordIndex {
    fn default() -> Self {
        Self { direct_index: Default::default(), reverse_index: Default::default() }
    }
}

impl CoordIndex {
    pub fn add(&mut self, location: &Location) {
        if self.direct_index.get(location).is_none() {
            let value = self.direct_index.len();
            self.direct_index.insert(location.clone(), value);
            self.reverse_index.insert(value, location.clone());
        }
    }

    pub fn get_by_loc(&self, location: &Location) -> Option<usize> {
        self.direct_index.get(location).cloned()
    }

    pub fn get_by_idx(&self, index: &usize) -> Option<Location> {
        self.reverse_index.get(index).cloned()
    }

    #[allow(dead_code)]
    pub fn unique(&self) -> Vec<Location> {
        let mut sorted_pairs: Vec<_> = self.reverse_index.iter().collect();
        sorted_pairs.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        sorted_pairs.iter().map(|pair| pair.1.clone()).collect()
    }
}

impl Eq for Location {}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.lat == other.lat && self.lng == other.lng
    }
}

impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        write_hash(self.lat, state);
        write_hash(self.lng, state);
    }
}

fn write_hash<H: Hasher>(value: f64, state: &mut H) {
    let value: u64 = unsafe { std::mem::transmute(value) };
    state.write_u64(value);
}
