#[cfg(test)]
#[path = "../../../tests/unit/models/problem/jobs_test.rs"]
mod jobs_test;

use crate::models::common::*;
use crate::models::problem::{Costs, Fleet, TransportCost};
use hashbrown::HashMap;
use rosomaxa::prelude::compare_floats;
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

    /// Get all places from the job.
    pub fn places(&self) -> Box<dyn Iterator<Item = &Place> + '_> {
        match &self {
            Job::Single(single) => Box::new(single.places.iter()),
            Job::Multi(multi) => Box::new(multi.jobs.iter().flat_map(|single| single.places.iter())),
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
    pub fn new_shared(jobs: Vec<Arc<Single>>, dimens: Dimensions) -> Arc<Self> {
        let permutations = vec![(0..jobs.len()).collect()];
        Self::bind(Self { jobs, dimens, permutator: Box::new(FixedJobPermutation::new(permutations)) })
    }

    /// Creates a new multi job from given 'dimens' and `jobs` using `permutator` to control insertion order.
    pub fn new_shared_with_permutator(
        jobs: Vec<Arc<Single>>,
        dimens: Dimensions,
        permutator: Box<dyn JobPermutation + Send + Sync>,
    ) -> Arc<Self> {
        Self::bind(Self { jobs, dimens, permutator })
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

    /// Returns parent multi job for given sub-job.
    pub fn roots(single: &Single) -> Option<Arc<Multi>> {
        single.dimens.get_value::<Weak<Multi>>("rf").and_then(|w| w.upgrade())
    }

    /// Wraps given multi job into [`Arc`] adding reference to it from all sub-jobs.
    fn bind(mut multi: Self) -> Arc<Self> {
        Arc::new_cyclic(|weak_multi| {
            multi.jobs.iter_mut().for_each(|single| {
                Arc::get_mut(single)
                    .expect("Single from Multi should not be shared before binding")
                    .dimens
                    .set_value("rf", weak_multi.clone());
            });

            multi
        })
    }
}

type JobIndex = HashMap<Job, (Vec<(Job, Cost)>, HashMap<Job, Cost>, Cost)>;

/// Stores all jobs taking into account their neighborhood.
pub struct Jobs {
    jobs: Vec<Job>,
    index: HashMap<usize, JobIndex>,
}

impl Jobs {
    /// Creates a new [`Jobs`].
    pub fn new(fleet: &Fleet, jobs: Vec<Job>, transport: &Arc<dyn TransportCost + Send + Sync>) -> Jobs {
        Jobs { jobs: jobs.clone(), index: create_index(fleet, jobs, transport) }
    }

    /// Returns all jobs in original order.
    pub fn all(&'_ self) -> impl Iterator<Item = Job> + '_ {
        self.jobs.iter().cloned()
    }

    /// Returns range of jobs "near" to given one. Near is defined by costs with relation
    /// transport profile and departure time.
    pub fn neighbors(&self, profile: &Profile, job: &Job, _: Timestamp) -> impl Iterator<Item = &(Job, Cost)> {
        self.index.get(&profile.index).unwrap().get(job).unwrap().0.iter()
    }

    /// Returns cost distance between two jobs.
    pub fn distance(&self, profile: &Profile, from: &Job, to: &Job, _: Timestamp) -> Cost {
        *self.index.get(&profile.index).unwrap().get(from).unwrap().1.get(to).unwrap()
    }

    /// Returns job rank as relative cost from any vehicle's start position.
    pub fn rank(&self, profile: &Profile, job: &Job) -> Cost {
        self.index.get(&profile.index).unwrap().get(job).unwrap().2
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
            (Job::Single(lhs), Job::Single(rhs)) => std::ptr::eq(lhs.as_ref(), rhs.as_ref()),
            (Job::Multi(lhs), Job::Multi(rhs)) => std::ptr::eq(lhs.as_ref(), rhs.as_ref()),
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

/// Returns job locations.
pub fn get_job_locations<'a>(job: &'a Job) -> Box<dyn Iterator<Item = Option<Location>> + 'a> {
    match job {
        Job::Single(single) => Box::new(single.places.iter().map(|p| p.location)),
        Job::Multi(multi) => Box::new(multi.jobs.iter().flat_map(|j| j.places.iter().map(|p| p.location))),
    }
}

// TODO: we don't know actual departure and zero-cost when we create job index.
const DEFAULT_COST: Cost = 0.;
const UNREACHABLE_COST: Cost = f64::MAX;

/// Creates job index.
fn create_index(
    fleet: &Fleet,
    jobs: Vec<Job>,
    transport: &Arc<dyn TransportCost + Send + Sync>,
) -> HashMap<usize, JobIndex> {
    let avg_profile_costs = get_avg_profile_costs(fleet);

    fleet.profiles.iter().fold(HashMap::new(), |mut acc, profile| {
        let avg_costs = avg_profile_costs.get(&profile.index).unwrap();
        // get all possible start positions for given profile
        let starts: Vec<Location> = fleet
            .vehicles
            .iter()
            .filter(|v| v.profile.index == profile.index)
            .flat_map(|v| v.details.iter().map(|d| d.start.as_ref().map(|s| s.location)))
            .flatten()
            .collect();

        // create job index
        let item = jobs.iter().cloned().fold(HashMap::new(), |mut acc, job| {
            let mut sorted_job_costs: Vec<(Job, Cost)> = jobs
                .iter()
                .filter(|j| **j != job)
                .map(|j| (j.clone(), get_cost_between_jobs(profile, avg_costs, transport.as_ref(), &job, j)))
                .collect();
            sorted_job_costs.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Less));

            let fleet_costs = starts
                .iter()
                .cloned()
                .map(|s| get_cost_between_job_and_location(profile, avg_costs, transport.as_ref(), &job, s))
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
                .unwrap_or(DEFAULT_COST);

            let job_costs_map = sorted_job_costs.iter().cloned().collect::<HashMap<_, _>>();

            acc.insert(job, (sorted_job_costs, job_costs_map, fleet_costs));
            acc
        });

        acc.insert(profile.index, item);
        acc
    })
}

fn get_cost_between_locations(
    profile: &Profile,
    costs: &Costs,
    transport: &(dyn TransportCost + Send + Sync),
    from: Location,
    to: Location,
) -> f64 {
    let distance = transport.distance_approx(profile, from, to);
    let duration = transport.duration_approx(profile, from, to);

    if distance < 0. || duration < 0. {
        // NOTE this happens if matrix uses negative values as a marker of unreachable location
        UNREACHABLE_COST
    } else {
        distance * costs.per_distance + duration * costs.per_driving_time
    }
}

/// Returns min cost between job and location.
fn get_cost_between_job_and_location(
    profile: &Profile,
    costs: &Costs,
    transport: &(dyn TransportCost + Send + Sync),
    job: &Job,
    to: Location,
) -> Cost {
    get_job_locations(job)
        .map(|from| match from {
            Some(from) => get_cost_between_locations(profile, costs, transport, from, to),
            _ => DEFAULT_COST,
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Less))
        .unwrap_or(DEFAULT_COST)
}

/// Returns minimal cost between jobs.
fn get_cost_between_jobs(
    profile: &Profile,
    costs: &Costs,
    transport: &(dyn TransportCost + Send + Sync),
    lhs: &Job,
    rhs: &Job,
) -> f64 {
    let outer: Vec<Option<Location>> = get_job_locations(lhs).collect();
    let inner: Vec<Option<Location>> = get_job_locations(rhs).collect();

    let (location_cost, duration) = outer
        .iter()
        .flat_map(|o| inner.iter().map(move |i| (*o, *i)))
        .map(|pair| match pair {
            (Some(from), Some(to)) => {
                let total = get_cost_between_locations(profile, costs, transport, from, to);
                let duration = transport.duration_approx(profile, from, to);
                (total, duration)
            }
            _ => (DEFAULT_COST, 0.),
        })
        .min_by(|(a, _), (b, _)| compare_floats(*a, *b))
        .unwrap_or((DEFAULT_COST, 0.));

    let time_cost = lhs
        .places()
        .flat_map(|place| place.times.iter())
        .flat_map(|left| {
            rhs.places().flat_map(|place| place.times.iter()).map(move |right| match (left, right) {
                (TimeSpan::Window(left), TimeSpan::Window(right)) => {
                    // NOTE exclude traveling duration between jobs
                    (left.distance(right) - duration).max(0.)
                }
                _ => 0.,
            })
        })
        .min_by(|a, b| compare_floats(*a, *b))
        .unwrap_or(0.)
        * costs.per_waiting_time;

    location_cost + time_cost
}

fn get_avg_profile_costs(fleet: &Fleet) -> HashMap<usize, Costs> {
    let get_avg_by = |costs: &Vec<Costs>, map_cost_fn: fn(&Costs) -> f64| -> f64 {
        costs.iter().map(map_cost_fn).sum::<f64>() / (costs.len() as f64)
    };
    fleet
        .vehicles
        .iter()
        .fold(HashMap::new(), |mut acc, vehicle| {
            acc.entry(vehicle.profile.index).or_insert_with(Vec::new).push(vehicle.costs.clone());
            acc
        })
        .iter()
        .map(|(&profile_idx, costs)| {
            (
                profile_idx,
                Costs {
                    fixed: get_avg_by(costs, |c| c.fixed),
                    per_distance: get_avg_by(costs, |c| c.per_distance),
                    per_driving_time: get_avg_by(costs, |c| c.per_driving_time),
                    per_waiting_time: get_avg_by(costs, |c| c.per_waiting_time),
                    per_service_time: get_avg_by(costs, |c| c.per_service_time),
                },
            )
        })
        .collect()
}
