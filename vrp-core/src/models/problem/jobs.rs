#[cfg(test)]
#[path = "../../../tests/unit/models/problem/jobs_test.rs"]
mod jobs_test;

use crate::models::common::*;
use crate::models::problem::{Costs, Fleet, TransportCost};
use hashbrown::HashMap;
use std::cell::UnsafeCell;
use std::cmp::Ordering::Less;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// Represents a job variant.
#[derive(Clone)]
pub enum Job {
    /// Single job.
    Single(Arc<Single>),
    /// MultiJob with multiple dependent jobs.
    Multi(Arc<Multi>),
}

impl Job {
    /// Considers job as [`Single`].
    pub fn as_single(&self) -> Option<&Arc<Single>> {
        match &self {
            Job::Single(job) => Some(job),
            _ => None,
        }
    }

    /// Considers job as [`Single`]. Panics if it is [`Multi`].
    pub fn to_single(&self) -> &Arc<Single> {
        self.as_single().expect("Unexpected job type: multi")
    }

    /// Considers job as [`Multi`].
    pub fn as_multi(&self) -> Option<&Arc<Multi>> {
        match &self {
            Job::Multi(job) => Some(job),
            _ => None,
        }
    }

    /// Considers job as [`Multi`]. Panics if it is [`Multi`]
    pub fn to_multi(&self) -> &Arc<Multi> {
        self.as_multi().expect("Unexpected job type: single")
    }

    /// Returns dimensions collection.
    pub fn dimens(&self) -> &Dimensions {
        match &self {
            Job::Single(single) => &single.dimens,
            Job::Multi(multi) => &multi.dimens,
        }
    }
}

/// Represents a job place details where and/or when work has to be performed.
#[derive(Clone)]
pub struct Place {
    /// Location where work has to be performed.
    pub location: Option<Location>,
    /// Time has to be spend performing work.
    pub duration: Duration,
    /// Time data which specifies when work can be started.
    pub times: Vec<TimeSpan>,
}

/// Represents a job which should be performed once but actual place/time might vary.
pub struct Single {
    /// Specifies job details: where and when it can be performed.
    pub places: Vec<Place>,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
}

/// Represents a job which consists of multiple sub jobs.
/// All of these jobs must be performed or none of them. Order can be controlled
/// via specific dimension value.
pub struct Multi {
    /// A list of jobs which must be performed.
    pub jobs: Vec<Arc<Single>>,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
    /// Permutation generator.
    permutator: Box<dyn JobPermutation + Send + Sync>,
}

/// Defines a trait to work with multi job's permutations.
pub trait JobPermutation {
    // TODO fix all implementations to support returning reference
    /// Returns a valid permutation.
    fn get(&self) -> Vec<Vec<usize>>;

    /// Validates given permutation.
    fn validate(&self, permutation: &[usize]) -> bool;
}

/// Specifies permutation generator which allows only fixed set of permutations.
pub struct FixedJobPermutation {
    permutations: Vec<Vec<usize>>,
}

impl FixedJobPermutation {
    /// Creates a new instance of `StrictJobPermutation`.
    pub fn new(permutations: Vec<Vec<usize>>) -> Self {
        Self { permutations }
    }
}

impl JobPermutation for FixedJobPermutation {
    fn get(&self) -> Vec<Vec<usize>> {
        self.permutations.clone()
    }

    fn validate(&self, permutation: &[usize]) -> bool {
        self.permutations
            .iter()
            .any(|prm| prm.len() == permutation.len() && prm.iter().zip(permutation.iter()).all(|(&a, &b)| a == b))
    }
}

impl Multi {
    /// Creates a new multi job from given 'dimens' and `jobs` assuming that jobs has to be
    /// inserted in order they specified.
    pub fn new(jobs: Vec<Arc<Single>>, dimens: Dimensions) -> Self {
        let permutations = vec![(0..jobs.len()).collect()];
        Self { jobs, dimens, permutator: Box::new(FixedJobPermutation::new(permutations)) }
    }

    /// Creates a new multi job from given 'dimens' and `jobs` using `permutator` to control insertion order.
    pub fn new_with_permutator(
        jobs: Vec<Arc<Single>>,
        dimens: Dimensions,
        permutator: Box<dyn JobPermutation + Send + Sync>,
    ) -> Self {
        Self { jobs, dimens, permutator }
    }

    /// Returns all sub-jobs permutations.
    pub fn permutations(&self) -> Vec<Vec<Arc<Single>>> {
        self.permutator
            .get()
            .iter()
            .map(|perm| perm.iter().map(|&i| self.jobs.get(i).unwrap().clone()).collect())
            .collect()
    }

    /// Validates given set of permutations.
    pub fn validate(&self, permutations: &[usize]) -> bool {
        self.permutator.validate(permutations)
    }

    /// Wraps given multi job into [`Arc`] adding reference to it from all sub-jobs.
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
            dimens.set_value("rf", weak_multi);
        });

        multi
    }

    /// Returns parent multi job for given sub-job.
    pub fn roots(single: &Single) -> Option<Arc<Multi>> {
        single.dimens.get_value::<Weak<Multi>>("rf").and_then(|w| w.upgrade())
    }
}

type JobIndex = HashMap<Job, (Vec<(Job, Cost)>, Cost)>;

/// Stores all jobs taking into account their neighborhood.
pub struct Jobs {
    jobs: Vec<Job>,
    index: HashMap<Profile, JobIndex>,
}

impl Jobs {
    /// Creates a new [`Jobs`].
    pub fn new(fleet: &Fleet, jobs: Vec<Job>, transport: &Arc<dyn TransportCost + Send + Sync>) -> Jobs {
        Jobs { jobs: jobs.clone(), index: create_index(fleet, jobs, transport) }
    }

    /// Returns all jobs in original order.
    pub fn all<'a>(&'a self) -> impl Iterator<Item = Job> + 'a {
        self.jobs.iter().cloned()
    }

    /// Returns range of jobs "near" to given one.Near is defined by transport costs,
    /// its profile and time. Value is filtered by max cost.
    pub fn neighbors<'a>(
        &'a self,
        profile: Profile,
        job: &Job,
        _: Timestamp,
        max_cost: Cost,
    ) -> impl Iterator<Item = Job> + 'a {
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
    pub fn rank(&self, profile: Profile, job: &Job) -> Cost {
        self.index.get(&profile).unwrap().get(job).unwrap().1
    }

    /// Returns amount of jobs.
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
fn create_index(
    fleet: &Fleet,
    jobs: Vec<Job>,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> HashMap<Profile, JobIndex> {
    let avg_profile_costs = get_avg_profile_costs(fleet);

    fleet.profiles.iter().cloned().fold(HashMap::new(), |mut acc, profile| {
        let avg_costs = avg_profile_costs.get(&profile).unwrap();
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
            let mut job_costs: Vec<(Job, Cost)> = jobs
                .iter()
                .filter(|j| **j != job)
                .map(|j| (j.clone(), get_cost_between_jobs(profile, avg_costs, transport, &job, j)))
                .collect();
            job_costs.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Less));

            let fleet_costs = starts
                .iter()
                .cloned()
                .map(|s| get_cost_between_job_and_location(profile, avg_costs, transport, &job, s))
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
fn get_cost_between_locations(
    profile: Profile,
    costs: &Costs,
    transport: &Arc<dyn TransportCost + Send + Sync>,
    from: Location,
    to: Location,
) -> f64 {
    transport.distance(profile, from, to, DEFAULT_DEPARTURE) * costs.per_distance
        + transport.duration(profile, from, to, DEFAULT_DEPARTURE) * costs.per_driving_time
}

/// Returns min cost between job and location.
fn get_cost_between_job_and_location(
    profile: Profile,
    costs: &Costs,
    transport: &Arc<dyn TransportCost + Send + Sync>,
    lhs: &Job,
    to: Location,
) -> Cost {
    get_job_locations(lhs)
        .map(|from| match from {
            Some(from) => get_cost_between_locations(profile, costs, transport, from, to),
            _ => DEFAULT_COST,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_COST)
}

/// Returns minimal cost between jobs.
fn get_cost_between_jobs(
    profile: Profile,
    costs: &Costs,
    transport: &Arc<dyn TransportCost + Send + Sync>,
    lhs: &Job,
    rhs: &Job,
) -> f64 {
    let outer: Vec<Option<Location>> = get_job_locations(lhs).collect();
    let inner: Vec<Option<Location>> = get_job_locations(rhs).collect();

    outer
        .iter()
        .flat_map(|o| inner.iter().map(move |i| (*o, *i)))
        .map(|pair| match pair {
            (Some(from), Some(to)) => get_cost_between_locations(profile, costs, transport, from, to),
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

fn get_avg_profile_costs(fleet: &Fleet) -> HashMap<Profile, Costs> {
    let get_avg_by = |costs: &Vec<Costs>, map_cost_fn: fn(&Costs) -> f64| -> f64 {
        costs.iter().map(map_cost_fn).sum::<f64>() / (costs.len() as f64)
    };
    fleet
        .vehicles
        .iter()
        .fold(HashMap::new(), |mut acc, vehicle| {
            acc.entry(vehicle.profile).or_insert_with(|| vec![]).push(vehicle.costs.clone());
            acc
        })
        .iter()
        .map(|(&profile, costs)| {
            (
                profile,
                Costs {
                    fixed: get_avg_by(&costs, |c| c.fixed),
                    per_distance: get_avg_by(&costs, |c| c.per_distance),
                    per_driving_time: get_avg_by(&costs, |c| c.per_driving_time),
                    per_waiting_time: get_avg_by(&costs, |c| c.per_waiting_time),
                    per_service_time: get_avg_by(&costs, |c| c.per_service_time),
                },
            )
        })
        .collect()
}
