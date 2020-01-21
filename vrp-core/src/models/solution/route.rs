use crate::models::common::{Duration, Location, Schedule, TimeWindow};
use crate::models::problem::{Actor, Job, Multi, Single};
use crate::models::solution::Tour;
use crate::utils::compare_shared;
use std::hash::{Hash, Hasher};
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
    pub job: Option<Arc<Single>>,
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

impl PartialEq<Route> for Route {
    fn eq(&self, other: &Route) -> bool {
        &*self as *const Route == &*other as *const Route
    }
}

impl Eq for Route {}

impl Hash for Route {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = &*self as *const Route;
        address.hash(state);
    }
}

impl Activity {
    pub fn new_with_job(job: Arc<Single>) -> Self {
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

    pub fn has_same_job(&self, job: &Job) -> bool {
        match self.retrieve_job() {
            Some(j) => match (&j, job) {
                (Job::Multi(lhs), Job::Multi(rhs)) => compare_shared(&lhs, rhs),
                (Job::Single(lhs), Job::Single(rhs)) => compare_shared(&lhs, rhs),
                _ => false,
            },
            _ => false,
        }
    }

    pub fn retrieve_job(&self) -> Option<Job> {
        match self.job.as_ref() {
            Some(single) => Multi::roots(single)
                .and_then(|multi| Some(Job::Multi(multi)))
                .or_else(|| Some(Job::Single(single.clone()))),
            _ => None,
        }
    }
}
