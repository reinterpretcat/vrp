use crate::models::common::Timestamp;
use crate::utils::compare_floats;
use std::any::Any;
use std::cmp::Ordering;
use std::collections::HashMap;

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

impl PartialEq<TimeWindow> for TimeWindow {
    fn eq(&self, other: &TimeWindow) -> bool {
        compare_floats(&self.start, &other.start) == Ordering::Equal
            && compare_floats(&self.end, &other.end) == Ordering::Equal
    }
}

impl Eq for TimeWindow {}

/// Represents a schedule.
#[derive(Clone, Debug)]
pub struct Schedule {
    pub arrival: Timestamp,
    pub departure: Timestamp,
}

impl PartialEq<Schedule> for Schedule {
    fn eq(&self, other: &Schedule) -> bool {
        compare_floats(&self.arrival, &other.arrival) == Ordering::Equal
            && compare_floats(&self.departure, &other.departure) == Ordering::Equal
    }
}

impl Eq for Schedule {}

/// Multiple named dimension which can represents anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
pub type Dimensions = HashMap<String, Box<dyn Any + Send + Sync>>;

/// Specifies size of requested work.
pub trait Size {}
