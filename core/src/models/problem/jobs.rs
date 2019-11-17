#[cfg(test)]
#[path = "../../../tests/unit/models/problem/jobs_test.rs"]
mod jobs_test;

use crate::models::common::*;
use crate::models::problem::{Fleet, TransportCost};
use std::cell::UnsafeCell;
use std::cmp::Ordering::Less;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// Represents a job variant.
pub enum Job {
    Single(Arc<Single>),
    Multi(Arc<Multi>),
}

impl Job {
    pub fn as_single(&self) -> Arc<Single> {
        match &self {
            Job::Single(job) => job.clone(),
            _ => panic!("Unexpected job type: multi"),
        }
    }

    pub fn as_multi(&self) -> Arc<Multi> {
        match &self {
            Job::Multi(job) => job.clone(),
            _ => panic!("Unexpected job type: single"),
        }
    }
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
    pub jobs: Vec<Arc<Single>>,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
    /// Permutation generator.
    generator: Box<dyn Fn(&Multi) -> Vec<Vec<usize>> + Send + Sync>,
}

impl Multi {
    pub fn new(jobs: Vec<Arc<Single>>, dimens: Dimensions) -> Self {
        Self { jobs, dimens, generator: Box::new(|m| vec![(0..m.jobs.len()).collect()]) }
    }

    pub fn new_with_generator(
        jobs: Vec<Arc<Single>>,
        dimens: Dimensions,
        generator: Box<dyn Fn(&Multi) -> Vec<Vec<usize>> + Send + Sync>,
    ) -> Self {
        Self { jobs, dimens, generator }
    }

    pub fn permutations(&self) -> Vec<Vec<Arc<Single>>> {
        (self.generator)(self)
            .iter()
            .map(|perm| perm.iter().map(|&i| self.jobs.get(i).unwrap().clone()).collect())
            .collect()
    }

    pub fn bind(multi: Self) -> Arc<Self> {
        // NOTE: layout must be identical
        struct SingleConstruct {
            pub places: UnsafeCell<Vec<Place>>,
            pub dimens: UnsafeCell<Dimensions>,
        }

        let multi = Arc::new(multi);

        multi.jobs.iter().for_each(|job| {
            let weak_multi = Arc::downgrade(&multi);
            let job: Arc<SingleConstruct> = unsafe { std::mem::transmute(job.clone()) };
            let dimens = unsafe { &mut *job.dimens.get() };
            dimens.insert("rf".to_owned(), Box::new(weak_multi));
        });

        multi
    }

    pub fn roots(single: &Single) -> Option<Arc<Multi>> {
        single.dimens.get("rf").map(|v| v.downcast_ref::<Weak<Multi>>()).and_then(|w| w).and_then(|w| w.upgrade())
    }
}

type JobIndex = HashMap<Arc<Job>, (Vec<(Arc<Job>, Cost)>, Cost)>;

/// Stores all jobs taking into account their neighborhood.
pub struct Jobs {
    jobs: Vec<Arc<Job>>,
    index: HashMap<Profile, JobIndex>,
}

impl Jobs {
    pub fn new(fleet: &Fleet, jobs: Vec<Arc<Job>>, transport: &impl TransportCost) -> Jobs {
        Jobs { jobs: jobs.clone(), index: create_index(fleet, jobs, transport) }
    }

    pub fn all<'a>(&'a self) -> impl Iterator<Item = Arc<Job>> + 'a {
        self.jobs.iter().cloned()
    }

    /// Returns range of jobs "near" to given one.Near is defined by transport costs,
    /// its profile and time. Value is filtered by max cost.
    pub fn neighbors<'a>(
        &'a self,
        profile: Profile,
        job: &Arc<Job>,
        _: Timestamp,
        max_cost: Cost,
    ) -> impl Iterator<Item = Arc<Job>> + 'a {
        self.index
            .get(&profile)
            .unwrap()
            .get(job)
            .unwrap()
            .0
            .iter()
            .filter(move |(_, cost)| *cost > 0. && *cost < max_cost)
            .map(|(j, _)| j.clone())
    }

    /// Returns job rank as relative cost from any vehicle's start position.
    pub fn rank(&self, profile: Profile, job: &Arc<Job>) -> Cost {
        self.index.get(&profile).unwrap().get(job).unwrap().1
    }

    /// Returns amount of jobs
    pub fn size(&self) -> usize {
        self.jobs.len()
    }
}

impl PartialEq<Job> for Job {
    fn eq(&self, other: &Job) -> bool {
        match (&self, other) {
            (Job::Single(_), Job::Multi(_)) => false,
            (Job::Multi(_), Job::Single(_)) => false,
            (Job::Single(lhs), Job::Single(rhs)) => lhs.as_ref() as *const Single == rhs.as_ref() as *const Single,
            (Job::Multi(lhs), Job::Multi(rhs)) => lhs.as_ref() as *const Multi == rhs.as_ref() as *const Multi,
        }
    }
}

impl Eq for Job {}

impl Hash for Job {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Job::Single(single) => {
                let address = single.as_ref() as *const Single;
                address.hash(state);
            }
            Job::Multi(multi) => {
                let address = multi.as_ref() as *const Multi;
                address.hash(state);
            }
        }
    }
}

// TODO: we don't know actual departure and zero-cost when we create job index.
const DEFAULT_DEPARTURE: Timestamp = 0.0;
const DEFAULT_COST: Cost = 0.0;

/// Creates job index.
fn create_index(fleet: &Fleet, jobs: Vec<Arc<Job>>, transport: &impl TransportCost) -> HashMap<Profile, JobIndex> {
    fleet.profiles.iter().cloned().fold(HashMap::new(), |mut acc, profile| {
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
            let mut job_costs: Vec<(Arc<Job>, Cost)> = jobs
                .iter()
                .filter(|j| j.as_ref() != job.as_ref())
                .map(|j| (j.clone(), get_cost_between_jobs(profile, transport, &job, j)))
                .collect();
            job_costs.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Less));

            let fleet_costs = starts
                .iter()
                .cloned()
                .map(|s| get_cost_between_job_and_location(profile, transport, &job, s))
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
                .unwrap_or(DEFAULT_COST);

            acc.insert(job, (job_costs, fleet_costs));
            acc
        });

        acc.insert(profile, item);
        acc
    })
}

#[inline(always)]
fn get_cost_between_locations(profile: Profile, transport: &impl TransportCost, from: Location, to: Location) -> f64 {
    transport.distance(profile, from, to, DEFAULT_DEPARTURE) + transport.duration(profile, from, to, DEFAULT_DEPARTURE)
}

/// Returns min cost between job and location.
fn get_cost_between_job_and_location(
    profile: Profile,
    transport: &impl TransportCost,
    lhs: &Job,
    to: Location,
) -> Cost {
    get_job_locations(lhs)
        .map(|from| match from {
            Some(from) => get_cost_between_locations(profile, transport, from, to),
            _ => DEFAULT_COST,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_COST)
}

/// Returns minimal cost between jobs.
fn get_cost_between_jobs(profile: Profile, transport: &impl TransportCost, lhs: &Job, rhs: &Job) -> f64 {
    let outer: Vec<Option<Location>> = get_job_locations(lhs).collect();
    let inner: Vec<Option<Location>> = get_job_locations(rhs).collect();

    outer
        .iter()
        .flat_map(|o| inner.iter().map(move |i| (*o, *i)))
        .map(|pair| match pair {
            (Some(from), Some(to)) => get_cost_between_locations(profile, transport, from, to),
            _ => DEFAULT_COST,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_COST)
}

/// Returns job locations.
fn get_job_locations<'a>(job: &'a Job) -> Box<dyn Iterator<Item = Option<Location>> + 'a> {
    match job {
        Job::Single(single) => Box::new(single.places.iter().map(|p| p.location)),
        Job::Multi(multi) => Box::new(multi.jobs.iter().flat_map(|j| j.places.iter().map(|p| p.location))),
    }
}
