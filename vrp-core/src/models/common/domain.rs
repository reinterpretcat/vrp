use crate::models::common::Timestamp;
use crate::utils::compare_floats;
use hashbrown::HashMap;
use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Specifies location type.
pub type Location = usize;

/// Represents a routing profile.
pub type Profile = i32;

/// Represents a time window.
#[derive(Clone, Debug)]
pub struct TimeWindow {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl TimeWindow {
    /// Creates a new [`TimeWindow`].
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Returns unlimited time window.
    pub fn max() -> Self {
        Self { start: 0., end: std::f64::MAX }
    }
}

impl PartialEq<TimeWindow> for TimeWindow {
    fn eq(&self, other: &TimeWindow) -> bool {
        compare_floats(self.start, other.start) == Ordering::Equal
            && compare_floats(self.end, other.end) == Ordering::Equal
    }
}

impl Eq for TimeWindow {}

impl Hash for TimeWindow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let start: i64 = unsafe { std::mem::transmute(self.start) };
        let end: i64 = unsafe { std::mem::transmute(self.end) };

        start.hash(state);
        end.hash(state);
    }
}

/// Represents a schedule.
#[derive(Clone, Debug)]
pub struct Schedule {
    /// Arrival time.
    pub arrival: Timestamp,
    /// Departure time.
    pub departure: Timestamp,
}

impl Schedule {
    pub fn new(arrival: Timestamp, departure: Timestamp) -> Self {
        Self { arrival, departure }
    }
}

impl PartialEq<Schedule> for Schedule {
    fn eq(&self, other: &Schedule) -> bool {
        compare_floats(self.arrival, other.arrival) == Ordering::Equal
            && compare_floats(self.departure, other.departure) == Ordering::Equal
    }
}

impl Eq for Schedule {}

/// Multiple named dimensions which can contain anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
pub type Dimensions = HashMap<String, Box<dyn Any + Send + Sync>>;

/// A trait to return arbitrary typed value by its key.
pub trait ValueDimension {
    fn get_value<T: 'static>(&self, key: &str) -> Option<&T>;
}

impl ValueDimension for Dimensions {
    fn get_value<T: 'static>(&self, key: &str) -> Option<&T> {
        self.get(key).and_then(|any| any.downcast_ref::<T>())
    }
}

/// A trait to get or set id.
pub trait IdDimension {
    fn set_id(&mut self, id: &str) -> &mut Self;
    fn get_id(&self) -> Option<&String>;
}

impl IdDimension for Dimensions {
    fn set_id(&mut self, id: &str) -> &mut Self {
        self.insert("id".to_string(), Box::new(id.to_string()));
        self
    }

    fn get_id(&self) -> Option<&String> {
        self.get_value::<String>("id")
    }
}
