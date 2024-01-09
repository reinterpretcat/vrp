#[cfg(test)]
#[path = "../../../tests/unit/models/common/domain_test.rs"]
mod domain_test;

use crate::models::common::{Duration, Timestamp};
use hashbrown::HashMap;
use rosomaxa::prelude::compare_floats;
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

    /// Checks whether time window has intersection with another one (inclusive).
    pub fn intersects(&self, other: &Self) -> bool {
        compare_floats(self.start, other.end) != Ordering::Greater
            && compare_floats(other.start, self.end) != Ordering::Greater
    }

    /// Checks whether time window has intersection with another one (exclusive).
    pub fn intersects_exclusive(&self, other: &Self) -> bool {
        compare_floats(self.start, other.end) == Ordering::Less
            && compare_floats(other.start, self.end) == Ordering::Less
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

/// A key which distinguishes different dimensions. A dimension is an extension mechanism which is used to
/// associate arbitrary data with an entity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DimenKey(usize);

/// A dimension scope used for key generation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DimenScope {
    /// A dimension key to store information associated with activity (job).
    Activity,
    /// A dimension key to store information associated with vehicle (actor).
    Vehicle,
}

/// A registry (factory) to produce unique dimension keys.
#[derive(Debug, Default)]
pub struct DimenKeyRegistry {
    keys_data: HashMap<DimenScope, usize>,
}

impl DimenKeyRegistry {
    /// Gets next dimension key for given scope.
    pub fn next_key(&mut self, scope: DimenScope) -> DimenKey {
        // NOTE start from 1, reserving 0 for id
        let id = self.keys_data.entry(scope).and_modify(|counter| *counter += 1).or_insert(1);

        DimenKey(*id)
    }
}

/// A core dimensions which used internally to resolve some core properties.
pub(crate) struct CoreDimensions {
    /// Id of an entity.
    pub id: Option<String>,
    /// An extension property.
    pub extension: Option<Arc<dyn Any + Send + Sync>>,
}

/// Multiple named dimensions which can contain anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
#[derive(Clone, Default)]
pub struct Dimensions {
    data: Vec<Arc<dyn Any + Send + Sync>>,
}

impl Dimensions {
    /// Gets unique id as a string reference.
    pub fn get_id(&self) -> Option<&String> {
        self.get_value::<CoreDimensions>(DimenKey(0)).and_then(|core_dimens| core_dimens.id.as_ref())
    }

    /// Sets id.
    pub fn set_id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let extension = if let Some(core_dimens) = self.get_value::<CoreDimensions>(DimenKey(0)) {
            core_dimens.extension.clone()
        } else {
            None
        };

        let core_dimens = CoreDimensions { id: Some(id.as_ref().to_string()), extension };
        self.set_value(DimenKey(0), Arc::new(core_dimens));
        self
    }

    /// Gets an arbitrary value which can be extracted without dimension key.
    pub(crate) fn get_extension<T: 'static>(&self) -> Option<&T> {
        self.get_value::<CoreDimensions>(DimenKey(0))
            .and_then(|core_dimens| core_dimens.extension.as_ref())
            .and_then(|any| any.downcast_ref::<T>())
    }

    /// Gets value associated with given key.
    pub fn get_value<T: 'static>(&self, key: DimenKey) -> Option<&T> {
        self.data.get(key.0).and_then(|any| any.downcast_ref::<T>())
    }

    /// Sets value associated with given key.
    /// Returns whether the value was set.
    pub fn set_value<T: 'static + Sync + Send>(&mut self, key: DimenKey, value: T) -> bool {
        // NOTE: this shouldn't be possible as we control distribution of dimension keys
        debug_assert_ne!(key.0, 0, "attempt to override reserved dimension");

        if let Some(entry) = self.data.get_mut(key.0) {
            *entry = Arc::new(value);
            true
        } else {
            false
        }
    }

    /// Sets an extension value which can be used without dimension key.
    /// NOTE: only one value per dimension can be used.
    pub(crate) fn set_extension<T: 'static + Send + Sync>(&mut self, extension: T) {
        let id = if let Some(core_dimens) = self.get_value::<CoreDimensions>(DimenKey(0)) {
            core_dimens.id.clone()
        } else {
            None
        };

        self.set_value(DimenKey(0), CoreDimensions { id, extension: Some(Arc::new(extension)) });
    }
}
