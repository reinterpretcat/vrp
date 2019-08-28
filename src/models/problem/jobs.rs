#[cfg(test)]
#[path = "../../../tests/unit/models/problem/jobs_test.rs"]
mod jobs_test;

use crate::models::common::{
    Dimensions, Distance, Duration, Location, Profile, TimeWindow, Timestamp,
};
use crate::models::costs::TransportCost;
use crate::models::problem::Fleet;
use std::cmp::Ordering;
use std::cmp::Ordering::Less;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Represents a job variant.
pub enum Job {
    Single(Single),
    Multi(Multi),
}

/// Represents a job place details where and/or when work has to be performed.
pub struct Place {
    /// Location where work has to be performed.
    pub location: Option<Location>,
    /// Time has to be spend performing work.
    pub duration: Duration,
    /// Time windows when work can be started.
    pub times: Vec<TimeWindow>,
}

/// Represents a job which should be performed once but actual place/time might vary.
pub struct Single {
    /// Specifies job details: where and when it can be performed.
    pub places: Vec<Place>,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
}

/// Represents a job which consists of multiple sub jobs without ids.
/// All of these jobs must be performed or none of them. Order can be controlled
/// via specific dimension value.
pub struct Multi {
    /// A list of jobs which must be performed.
    pub jobs: Vec<Single>,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
}

type JobIndex = (Vec<(Arc<Job>, Distance)>, Distance);

/// Stores all jobs taking into account their neighborhood.
pub struct Jobs {
    jobs: Vec<Arc<Job>>,
    index: BTreeMap<String, JobIndex>,
}

impl Jobs {
    pub fn new(fleet: &Fleet, jobs: Vec<Job>) -> Jobs {
        Jobs {
            jobs: jobs.into_iter().map(|j| Arc::new(j)).collect(),
            index: Default::default(),
        }
    }

    pub fn all<'a>(&'a self) -> impl Iterator<Item = Arc<Job>> + 'a {
        self.jobs.iter().cloned()
    }
}

impl PartialEq<Job> for Job {
    fn eq(&self, other: &Job) -> bool {
        &*self as *const Job == &*other as *const Job
    }
}

impl Eq for Job {}

impl Hash for Job {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = &*self as *const Job;
        address.hash(state);
    }
}

// TODO: we don't know actual departure and zero-distance when we create job index.
const DEFAULT_DEPARTURE: Timestamp = 0.0;
const DEFAULT_DISTANCE: Distance = 0.0;

/// Creates job index.
fn create_index(
    fleet: &Fleet,
    jobs: Vec<Arc<Job>>,
    transport: impl TransportCost,
) -> HashMap<Profile, HashMap<Arc<Job>, (Vec<f64>, f64)>> {
    fleet
        .profiles
        .iter()
        .cloned()
        .fold(HashMap::new(), |mut acc, profile| {
            // get all possible start positions for given profile
            let starts: Vec<Location> = fleet
                .vehicles
                .iter()
                .filter(|v| v.profile == profile)
                .flat_map(|v| v.details.iter().map(|d| d.start))
                .filter(|s| s.is_some())
                .map(|s| s.unwrap())
                .collect();

            // create job index
            let item = jobs.iter().cloned().fold(HashMap::new(), |mut acc, job| {
                let mut job_distances: Vec<Distance> = jobs
                    .iter()
                    .filter(|j| j.as_ref() != job.as_ref())
                    .map(|j| get_distance_between_jobs(profile, &transport, j, &job))
                    .collect();
                job_distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Less));

                let fleet_distances = starts
                    .iter()
                    .cloned()
                    .map(|s| get_distance_between_job_and_location(profile, &transport, &job, s))
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
                    .unwrap_or(DEFAULT_DISTANCE);

                acc.insert(job, (job_distances, fleet_distances));
                acc
            });

            acc.insert(profile, item);
            acc
        })
}

/// Returns min distance between job and location.
fn get_distance_between_job_and_location(
    profile: Profile,
    transport: &impl TransportCost,
    lhs: &Job,
    to: Location,
) -> Distance {
    get_job_locations(lhs)
        .map(|from| match from {
            Some(from) => transport.distance(profile, from, to, DEFAULT_DEPARTURE),
            _ => DEFAULT_DISTANCE,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_DISTANCE)
}

/// Returns minimal distance between jobs.
fn get_distance_between_jobs(
    profile: Profile,
    transport: &impl TransportCost,
    lhs: &Job,
    rhs: &Job,
) -> Distance {
    let outer: Vec<Option<Location>> = get_job_locations(lhs).collect();
    let inner: Vec<Option<Location>> = get_job_locations(rhs).collect();

    outer
        .iter()
        .flat_map(|o| inner.iter().map(move |i| (o.clone(), i.clone())))
        .map(|pair| match pair {
            (Some(from), Some(to)) => transport.distance(profile, from, to, DEFAULT_DEPARTURE),
            _ => DEFAULT_DISTANCE,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_DISTANCE)
}

/// Returns job locations.
fn get_job_locations<'a>(job: &'a Job) -> Box<dyn Iterator<Item = Option<Location>> + 'a> {
    match job {
        Job::Single(single) => Box::new(single.places.iter().map(|p| p.location)),
        Job::Multi(multi) => Box::new(
            multi
                .jobs
                .iter()
                .flat_map(|j| j.places.iter().map(|p| p.location)),
        ),
    }
}

mod comparators {
    pub struct CompareJobs {}
}
