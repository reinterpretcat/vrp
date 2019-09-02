use crate::models::common::{Duration, Location, Schedule, TimeWindow};
use crate::models::problem::Single;
use crate::models::solution::{Actor, Tour};
use std::sync::Arc;

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
    /// Specifies activity details.
    pub place: Place,

    /// Specifies activity's schedule: actual arrival and departure time.
    pub schedule: Schedule,

    /// Specifies job relation. Empty if it has no relation to single job (e.g. tour start or end).
    /// If single job is part of mutli job, then original job can be received via its dimens.
    pub job: Option<Arc<Single>>,
}

/// Represents a tour performing jobs.
pub struct Route {
    /// An actor associated within route.
    pub actor: Arc<Actor>,

    /// Specifies job tour assigned to this route.
    pub tour: Tour,
}
