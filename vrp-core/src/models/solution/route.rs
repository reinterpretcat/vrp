use crate::models::common::{Distance, Duration, Location, Schedule, TimeWindow};
use crate::models::problem::{Actor, Job, Multi, Single};
use crate::models::solution::Tour;
use crate::utils::short_type_name;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// Specifies an extra commute information to reach the actual place.
#[derive(Clone, Debug, Default)]
pub struct Commute {
    /// An commute information to reach place from other location.
    pub forward: CommuteInfo,

    /// An commute information to get out from the place to the next location.
    pub backward: CommuteInfo,
}

/// Commute information.
#[derive(Clone, Debug, Default)]
pub struct CommuteInfo {
    /// A previous or next location.
    pub location: Location,

    /// Travelled distance.
    pub distance: Distance,

    /// Travelled duration.
    pub duration: Duration,
}

/// Specifies activity place.
#[derive(Clone, Debug)]
pub struct Place {
    /// An index of the place in original job definition.
    pub idx: usize,

    /// Location where activity is performed.
    pub location: Location,

    /// Specifies activity's duration.
    pub duration: Duration,

    /// Specifies activity's time window: an interval when job is allowed to be started.
    pub time: TimeWindow,
}

/// Represents activity which is needed to be performed.
#[derive(Debug)]
pub struct Activity {
    /// Specifies activity details.
    pub place: Place,

    /// Specifies activity's schedule including commute time.
    pub schedule: Schedule,

    /// Specifies associated job. Empty if it has no association with a single job (e.g. tour start or end).
    /// If single job is part of multi job, then original job can be received via `retrieve_job` method.
    pub job: Option<Arc<Single>>,

    /// An extra commute time to the place.
    pub commute: Option<Commute>,
}

/// Represents a tour performing jobs.
pub struct Route {
    /// An actor associated within route.
    pub actor: Arc<Actor>,

    /// Specifies job tour assigned to this route.
    pub tour: Tour,
}

impl Route {
    /// Returns a deep copy of `Route`.
    pub fn deep_copy(&self) -> Self {
        Self { actor: self.actor.clone(), tour: self.tour.deep_copy() }
    }
}

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>()).field("actor", self.actor.as_ref()).field("tour", &self.tour).finish()
    }
}

impl Activity {
    /// Creates an activity with a job.
    pub fn new_with_job(job: Arc<Single>) -> Self {
        Activity {
            place: Place { idx: 0, location: 0, duration: 0, time: TimeWindow::max() },
            schedule: Schedule { arrival: 0, departure: 0 },
            job: Some(job),
            commute: None,
        }
    }

    /// Creates a deep copy of `Activity`.
    pub fn deep_copy(&self) -> Self {
        Self {
            place: Place {
                idx: self.place.idx,
                location: self.place.location,
                duration: self.place.duration,
                time: self.place.time.clone(),
            },
            schedule: self.schedule.clone(),
            job: self.job.clone(),
            commute: self.commute.clone(),
        }
    }

    /// Checks whether activity has given job.
    pub fn has_same_job(&self, job: &Job) -> bool {
        self.retrieve_job().as_ref().map_or(false, |other| other == job)
    }

    /// Returns job if activity has it.
    pub fn retrieve_job(&self) -> Option<Job> {
        match self.job.as_ref() {
            Some(single) => Multi::roots(single).map(Job::Multi).or_else(|| Some(Job::Single(single.clone()))),
            _ => None,
        }
    }
}

impl Commute {
    /// Checks whether there is no distance costs for commute.
    pub fn is_zero_distance(&self) -> bool {
        self.forward.is_zero_distance() & self.backward.is_zero_distance()
    }

    /// Gets total commute duration.
    pub fn duration(&self) -> Duration {
        self.forward.duration + self.backward.duration
    }
}

impl CommuteInfo {
    /// Checks whether there is no distance costs for part of commute.
    pub fn is_zero_distance(&self) -> bool {
        let is_zero_distance = self.distance == 0;

        if is_zero_distance && self.duration != 0 {
            unreachable!("expected to have duration to be zero, got: {}", self.duration);
        }

        is_zero_distance
    }
}
