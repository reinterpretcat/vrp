use crate::models::common::{Duration, Location, Schedule, TimeWindow};
use crate::models::problem::Single;

/// Specifies activity place.
pub struct Place {
    /// Location where activity is performed.
    pub location: Location,
    /// Specifies activity's duration.
    pub duration: Duration,
    /// Specifies activity's time window: an interval when job is allowed to be started.
    pub time: TimeWindow,
}

/// Represents activity which is needed to be performed.
pub struct Activity {
    place: Place,
    schedule: Schedule,
    job: Option<Single>,
}
