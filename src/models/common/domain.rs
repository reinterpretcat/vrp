use crate::models::common::Timestamp;
use std::any::Any;
use std::collections::HashMap;

/// Specifies location type.
pub type Location = usize;

/// Represents a routing profile.
pub type Profile = i32;

/// Represents a time window.
#[derive(Clone)]
pub struct TimeWindow {
    pub start: Timestamp,
    pub end: Timestamp,
}

/// Represents a schedule.
#[derive(Clone)]
pub struct Schedule {
    pub arrival: Timestamp,
    pub departure: Timestamp,
}

/// Multiple named dimension which can represents anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
pub type Dimensions = HashMap<String, Box<dyn Any>>;

/// Specifies size of requested work.
pub trait Size {}
