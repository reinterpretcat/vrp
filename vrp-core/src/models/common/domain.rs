#[cfg(test)]
#[path = "../../../tests/unit/models/common/domain_test.rs"]
mod domain_test;

use crate::models::common::{Duration, Timestamp};
use crate::utils::compare_floats;
use hashbrown::HashMap;
use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Specifies location type.
pub type Location = usize;

/// Represents a routing profile.
#[derive(Clone, Debug)]
pub struct Profile {
    /// An unique index.
    pub index: usize,
    /// A duration scale factor.
    pub scale: f64,
}

impl Profile {
    /// Creates a new instance of `Profile`.
    pub fn new(index: usize, scale: Option<f64>) -> Profile {
        Self { index, scale: scale.unwrap_or(1.) }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self { index: 0, scale: 1. }
    }
}

/// Specifies cost value.
pub type Cost = f64;

/// Represents a time window.
#[derive(Clone, Debug)]
pub struct TimeWindow {
    /// Start of time window.
    pub start: Timestamp,
    /// End of time window.
    pub end: Timestamp,
}

/// Represents a time offset.
#[derive(Clone, Debug)]
pub struct TimeOffset {
    /// Offset value to start time.
    pub start: Timestamp,
    /// Offset value to end time.
    pub end: Timestamp,
}

/// A enum for various time definitions.
#[derive(Clone, Debug)]
pub enum TimeSpan {
    /// A time window variant.
    Window(TimeWindow),
    /// A time offset variant.
    Offset(TimeOffset),
}

/// Specifies a flexible time interval.
#[derive(Clone, Debug, Default)]
pub struct TimeInterval {
    /// Earliest possible time to start.
    pub earliest: Option<Timestamp>,
    /// Latest possible time to stop.
    pub latest: Option<Timestamp>,
}

impl TimeWindow {
    /// Creates a new [`TimeWindow`].
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        Self { start, end }
    }

    /// Returns unlimited time window.
    pub fn max() -> Self {
        Self { start: 0., end: f64::MAX }
    }

    /// Checks whether time window has intersection with another one.
    pub fn intersects(&self, other: &Self) -> bool {
        compare_floats(self.start, other.end) != Ordering::Greater
            && compare_floats(other.start, self.end) != Ordering::Greater
    }

    /// Checks whether time window contains given time.
    pub fn contains(&self, time: Timestamp) -> bool {
        compare_floats(time, self.start) != Ordering::Less && compare_floats(time, self.end) != Ordering::Greater
    }

    /// Returns distance between two time windows.
    pub fn distance(&self, other: &Self) -> Timestamp {
        if self.intersects(other) {
            0.
        } else {
            // [other.s other.e] [self.s self.e]
            if self.start > other.start {
                self.start - other.end
            } else {
                // [self.s self.e] [other.s other.e]
                other.start - self.end
            }
        }
    }

    /// Returns a new overlapping time window.
    pub fn overlapping(&self, other: &Self) -> Option<TimeWindow> {
        if self.intersects(other) {
            let start = self.start.max(other.start);
            let end = self.end.min(other.end);

            Some(TimeWindow::new(start, end))
        } else {
            None
        }
    }

    /// Returns duration of time window.
    pub fn duration(&self) -> Duration {
        self.end - self.start
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

    /// Checks that this time span intersects with given time windows.
    pub fn intersects(&self, date: Timestamp, other: &TimeWindow) -> bool {
        self.to_time_window(date).intersects(other)
    }

    /// If time span is time window, then return it. Otherwise, return None.
    pub fn as_time_window(&self) -> Option<TimeWindow> {
        match &self {
            TimeSpan::Window(window) => Some(window.clone()),
            _ => None,
        }
    }
}

impl TimeInterval {
    /// Converts time interval to time window.
    pub fn to_time_window(&self) -> TimeWindow {
        TimeWindow { start: self.earliest.unwrap_or(0.), end: self.latest.unwrap_or(f64::MAX) }
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
    /// Creates a new instance of `Schedule`.
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
    /// Gets value from dimension with given key.
    fn get_value<T: 'static>(&self, key: &str) -> Option<&T>;
    /// Sets value in dimension with given key and value.
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
    /// Sets value as id.
    fn set_id(&mut self, id: &str) -> &mut Self;
    /// Gets id value if present.
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

impl Hash for TimeInterval {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let earliest = self.earliest.unwrap_or(0.).to_bits() as i64;
        let latest = self.latest.unwrap_or(f64::MAX).to_bits() as i64;

        earliest.hash(state);
        latest.hash(state);
    }
}

impl Eq for TimeInterval {}

impl PartialEq for TimeInterval {
    fn eq(&self, other: &Self) -> bool {
        self.earliest == other.earliest && self.latest == other.latest
    }
}
