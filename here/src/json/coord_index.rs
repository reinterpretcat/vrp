use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self { latitude, longitude }
    }
}

/// Represents coordinate index.
pub struct CoordIndex {
    direct_index: HashMap<Location, usize>,
    reverse_index: HashMap<usize, Location>,
}

impl CoordIndex {
    pub fn new() -> Self {
        Self { direct_index: Default::default(), reverse_index: Default::default() }
    }

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

    pub fn get_by_vec(&self, location: &Vec<f64>) -> Option<&usize> {
        assert_eq!(location.len(), 2);
        self.direct_index.get(&Location::new(*location.first().unwrap(), *location.last().unwrap()))
    }

    pub fn get_by_loc(&self, location: &Location) -> Option<&usize> {
        self.direct_index.get(location)
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
