use crate::models::common::{Duration, Location, Schedule, TimeWindow};
use crate::models::problem::{Job, Multi};
use crate::models::solution::{Actor, Tour};
use crate::utils::compare_shared;
use std::borrow::Borrow;
use std::sync::Arc;

/// Specifies activity place.
#[derive(Clone, Debug)]
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
    /// If single job is part of multi job, then original job can be received via its dimens.
    pub job: Option<Arc<Job>>,
}

/// Represents a tour performing jobs.
pub struct Route {
    /// An actor associated within route.
    pub actor: Arc<Actor>,

    /// Specifies job tour assigned to this route.
    pub tour: Tour,
}

impl Route {
    pub fn deep_copy(&self) -> Self {
        Self { actor: self.actor.clone(), tour: self.tour.deep_copy() }
    }
}

impl Activity {
    pub fn new_with_job(job: Arc<Job>) -> Self {
        Activity {
            place: Place { location: 0, duration: 0.0, time: TimeWindow { start: 0.0, end: std::f64::MAX } },
            schedule: Schedule { arrival: 0.0, departure: 0.0 },
            job: Some(job),
        }
    }

    pub fn deep_copy(&self) -> Self {
        Self {
            place: Place {
                location: self.place.location,
                duration: self.place.duration,
                time: self.place.time.clone(),
            },
            schedule: self.schedule.clone(),
            job: self.job.clone(),
        }
    }

    pub fn has_same_job(&self, job: &Arc<Job>) -> bool {
        match self.retrieve_job() {
            Some(j) => match (j.as_ref(), job.as_ref()) {
                (Job::Multi(lhs), Job::Multi(rhs)) => compare_shared(lhs, rhs),
                (Job::Single(lhs), Job::Single(rhs)) => compare_shared(lhs, rhs),
                _ => false,
            },
            _ => false,
        }
    }

    pub fn retrieve_job(&self) -> Option<Arc<Job>> {
        match self.job.borrow() {
            Some(job) => Some(match job.borrow() {
                Job::Single(single) => Multi::roots(single).map_or_else(|| job.clone(), |m| Arc::new(Job::Multi(m))),
                Job::Multi(_) => job.clone(),
            }),
            _ => None,
        }
    }
}
