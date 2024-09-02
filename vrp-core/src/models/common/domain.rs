#[cfg(test)]
#[path = "../../../tests/unit/models/common/domain_test.rs"]
mod domain_test;

use crate::models::common::{Duration, Timestamp};
use rosomaxa::prelude::Float;
use std::hash::{Hash, Hasher};

/// Specifies location type.
pub type Location = usize;

/// Represents a routing profile.
#[derive(Clone, Debug)]
pub struct Profile {
    /// An unique index.
    pub index: usize,
    /// A duration scale factor.
    pub scale: Float,
}

impl Profile {
    /// Creates a new instance of `Profile`.
    pub fn new(index: usize, scale: Option<Float>) -> Profile {
        Self { index, scale: scale.unwrap_or(1.) }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self { index: 0, scale: 1. }
    }
}

/// Specifies cost value.
pub type Cost = Float;

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
        Self { start: 0, end: Timestamp::MAX }
    }

    /// Checks whether time window has intersection with another one (inclusive).
    pub fn intersects(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    /// Checks whether time window has intersection with another one (exclusive).
    pub fn intersects_exclusive(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Checks whether time window contains given time.
    pub fn contains(&self, time: Timestamp) -> bool {
        time >= self.start && time <= self.end
    }

    /// Returns distance between two time windows.
    pub fn distance(&self, other: &Self) -> Duration {
        if self.intersects(other) {
            Duration::default()
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
        self.start == other.start && self.end == other.end
    }
}

impl Eq for TimeWindow {}

impl Hash for TimeWindow {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
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
        TimeWindow { start: self.earliest.unwrap_or_default(), end: self.latest.unwrap_or(Timestamp::MAX) }
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
        self.arrival == other.arrival && self.departure == other.departure
    }
}

impl Eq for Schedule {}

impl Hash for TimeInterval {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.earliest.unwrap_or_default().hash(state);
        self.latest.unwrap_or(Timestamp::MAX).hash(state);
    }
}

impl Eq for TimeInterval {}

impl PartialEq for TimeInterval {
    fn eq(&self, other: &Self) -> bool {
        self.earliest == other.earliest && self.latest == other.latest
    }
}
