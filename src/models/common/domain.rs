use crate::models::common::Timestamp;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

/// Specifies location type.
pub type Location = u64;

/// Represents a routing profile.
pub type Profile = String;

/// Represents a time window.
pub struct TimeWindow {
    start: Timestamp,
    end: Timestamp,
}

/// Represents a schedule.
pub struct Schedule {
    arrival: Timestamp,
    departure: Timestamp,
}

/// Multiple named dimension which can represents anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
pub type Dimensions = HashMap<String, Rc<dyn Any>>;
