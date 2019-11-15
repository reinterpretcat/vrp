use std::cmp::Ordering::Less;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }

    pub fn as_vec(&self) -> Vec<f64> {
        vec![self.latitude, self.longitude]
    }
}

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
    pub fn add_from_vec(&mut self, location: &Vec<f64>) {
        assert_eq!(location.len(), 2);
        self.add_from_loc(Location::new(*location.first().unwrap(), *location.last().unwrap()));
    }

    pub fn add_from_loc(&mut self, location: Location) {
        if self.direct_index.get(&location).is_none() {
            let value = self.direct_index.len();
            self.direct_index.insert(location.clone(), value);
            self.reverse_index.insert(value, location.clone());
        }
    }

    pub fn get_by_vec(&self, location: &Vec<f64>) -> Option<usize> {
        assert_eq!(location.len(), 2);
        self.direct_index.get(&Location::new(*location.first().unwrap(), *location.last().unwrap())).cloned()
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
        self.latitude == other.latitude && self.longitude == other.longitude
    }
}

impl Hash for Location {
    fn hash<H: Hasher>(&self, state: &mut H) {
        write_hash(self.latitude, state);
        write_hash(self.longitude, state);
    }
}

fn write_hash<H: Hasher>(value: f64, state: &mut H) {
    let value: u64 = unsafe { std::mem::transmute(value) };
    state.write_u64(value);
}
