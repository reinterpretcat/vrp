use crate::models::common::Timestamp;
use crate::utils::compare_floats;
use hashbrown::HashMap;
use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Specifies location type.
pub type Location = usize;

/// Represents a routing profile.
pub type Profile = i32;

/// Specifies cost value.
pub type Cost = f64;

/// Represents a time window.
#[derive(Clone, Debug)]
pub struct TimeWindow {
    pub start: Timestamp,
    pub end: Timestamp,
}

/// Represents a time offset.
#[derive(Clone, Debug)]
pub struct TimeOffset {
    pub start: Timestamp,
    pub end: Timestamp,
}

/// A enum for various time definitions.
#[derive(Clone, Debug)]
pub enum TimeSpan {
    Window(TimeWindow),
    Offset(TimeOffset),
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

    /// Checks whether time window has intersection with another one.
    pub fn intersects(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Checks whether time window has intersection with all others.
    pub fn intersects_all(&self, others: &[Self]) -> bool {
        others.iter().all(|other| other.intersects(self))
    }

    /// Checks whether time window has intersection with any of others.
    pub fn intersects_any(&self, others: &[Self]) -> bool {
        others.iter().any(|other| other.intersects(self))
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
        let start = self.start.to_bits() as i64;
        let end = self.end.to_bits() as i64;

        start.hash(state);
        end.hash(state);
    }
}

impl TimeOffset {
    /// Creates a new [`TimeOffset`].
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }
}

impl TimeSpan {
    /// Converts given time span into time window.
    pub fn to_time_window(&self, date: Timestamp) -> TimeWindow {
        match &self {
            TimeSpan::Window(window) => window.clone(),
            TimeSpan::Offset(offset) => TimeWindow::new(date + offset.start, date + offset.end),
        }
    }

    pub fn intersects(&self, date: Timestamp, other: &TimeWindow) -> bool {
        self.to_time_window(date).intersects(other)
    }

    pub fn as_time_window(&self) -> Option<TimeWindow> {
        match &self {
            TimeSpan::Window(window) => Some(window.clone()),
            _ => None,
        }
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
pub type Dimensions = HashMap<String, Arc<dyn Any + Send + Sync>>;

/// A trait to return arbitrary typed value by its key.
pub trait ValueDimension {
    fn get_value<T: 'static>(&self, key: &str) -> Option<&T>;
    fn set_value<T: 'static + Sync + Send>(&mut self, key: &str, value: T);
}

impl ValueDimension for Dimensions {
    fn get_value<T: 'static>(&self, key: &str) -> Option<&T> {
        self.get(key).and_then(|any| any.downcast_ref::<T>())
    }

    fn set_value<T: 'static + Sync + Send>(&mut self, key: &str, value: T) {
        self.insert(key.to_owned(), Arc::new(value));
    }
}

/// A trait to get or set id.
pub trait IdDimension {
    fn set_id(&mut self, id: &str) -> &mut Self;
    fn get_id(&self) -> Option<&String>;
}

impl IdDimension for Dimensions {
    fn set_id(&mut self, id: &str) -> &mut Self {
        self.set_value("id", id.to_string());
        self
    }

    fn get_id(&self) -> Option<&String> {
        self.get_value("id")
    }
}
