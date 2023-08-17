#[cfg(test)]
#[path = "../../../tests/unit/models/solution/tour_test.rs"]
mod tour_test;

use crate::models::common::{IdDimension, Schedule};
use crate::models::problem::{Actor, Job};
use crate::models::solution::{Activity, Place};
use crate::models::OP_START_MSG;
use crate::utils::{short_type_name, Either};
use hashbrown::HashSet;
use rustc_hash::FxHasher;
use std::fmt::{Debug, Formatter};
use std::hash::BuildHasherDefault;
use std::iter::once;
use std::slice::{Iter, IterMut};

/// A tour leg.
pub type Leg<'a> = (&'a [Activity], usize);

/// Represents a tour, a smart container for jobs with their associated activities.
#[derive(Default)]
pub struct Tour {
    /// Stores activities in the order the performed.
    activities: Vec<Activity>,

    /// Stores jobs in the order of their activities added.
    jobs: HashSet<Job, BuildHasherDefault<FxHasher>>,

    /// Keeps track whether tour is set as closed.
    is_closed: bool,
}

impl Tour {
    /// Creates a new tour with start and optional end using actor properties.
    pub fn new(actor: &Actor) -> Self {
        let mut tour = Self::default();
        tour.set_start(create_start_activity(actor));
        create_end_activity(actor).map(|end| tour.set_end(end));

        tour
    }

    /// Sets tour start.
    pub fn set_start(&mut self, activity: Activity) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(self.activities.is_empty());
        self.activities.push(activity);

        self
    }

    /// Sets tour end.
    pub fn set_end(&mut self, activity: Activity) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(!self.activities.is_empty());
        self.activities.push(activity);
        self.is_closed = true;

        self
    }

    /// Inserts activity within its job to the end of tour.
    pub fn insert_last(&mut self, activity: Activity) -> &mut Tour {
        self.insert_at(activity, self.job_activity_count() + 1);
        self
    }

    /// Inserts activity within its job at specified index.
    pub fn insert_at(&mut self, activity: Activity, index: usize) -> &mut Tour {
        assert!(activity.job.is_some());
        assert!(!self.activities.is_empty());

        self.jobs.insert(activity.retrieve_job().unwrap());
        self.activities.insert(index, activity);

        self
    }

    /// Removes job within its activities from the tour.
    pub fn remove(&mut self, job: &Job) -> bool {
        self.activities.retain(|a| !a.has_same_job(job));
        self.jobs.remove(job)
    }

    /// Removes activity and its job from the tour.
    pub fn remove_activity_at(&mut self, idx: usize) -> Job {
        let job = self
            .activities
            .get(idx)
            .and_then(|a| a.retrieve_job())
            .expect("Attempt to remove activity without job from the tour!");
        self.remove(&job);

        job
    }

    /// Returns all activities in tour.
    pub fn all_activities(&self) -> Iter<Activity> {
        self.activities.iter()
    }

    /// Returns activities slice in specific range (all inclusive).
    pub fn activities_slice(&self, start: usize, end: usize) -> &[Activity] {
        &self.activities[start..=end]
    }

    /// Returns all activities in tour as mutable.
    pub fn all_activities_mut(&mut self) -> IterMut<Activity> {
        self.activities.iter_mut()
    }

    /// Returns all activities in tour for specific job.
    pub fn job_activities<'a>(&'a self, job: &'a Job) -> impl Iterator<Item = &Activity> + 'a {
        self.activities.iter().filter(move |a| a.has_same_job(job))
    }

    /// Returns counted tour legs.
    pub fn legs(&self) -> impl Iterator<Item = Leg<'_>> + '_ + Clone {
        let last_index = if self.activities.is_empty() { 0 } else { self.activities.len() - 1 };

        let window_size = if self.activities.len() == 1 { 1 } else { 2 };
        let legs = self.activities.windows(window_size).zip(0_usize..);

        let is_open_tour_with_jobs = !self.is_closed && last_index > 0;

        if is_open_tour_with_jobs {
            Either::Left(legs.chain(once((&self.activities[last_index..], last_index))))
        } else {
            Either::Right(legs)
        }
    }

    /// Returns all jobs.
    pub fn jobs(&'_ self) -> impl Iterator<Item = &Job> + '_ {
        self.jobs.iter()
    }

    /// Returns activity by its index in tour.
    pub fn get(&self, index: usize) -> Option<&Activity> {
        self.activities.get(index)
    }

    /// Returns mutable activity by its index in tour.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Activity> {
        self.activities.get_mut(index)
    }

    /// Returns start activity in tour.
    pub fn start(&self) -> Option<&Activity> {
        self.activities.first()
    }

    /// Returns end activity in tour.
    pub fn end(&self) -> Option<&Activity> {
        self.activities.last()
    }

    /// Checks whether job is present in tour
    pub fn contains(&self, job: &Job) -> bool {
        self.jobs.contains(job)
    }

    /// Returns index of first job occurrence in the tour.
    pub fn index(&self, job: &Job) -> Option<usize> {
        self.activities.iter().position(move |a| a.has_same_job(job))
    }

    /// Checks whether job is present in tour.
    pub fn has_job(&self, job: &Job) -> bool {
        self.jobs.contains(job)
    }

    /// Checks whether tour has jobs.
    pub fn has_jobs(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Returns total amount of job activities.
    pub fn job_activity_count(&self) -> usize {
        if self.activities.is_empty() {
            0
        } else {
            self.activities.len() - (if self.is_closed { 2 } else { 1 })
        }
    }

    /// Returns amount of all activities in tour.
    pub fn total(&self) -> usize {
        self.activities.len()
    }

    /// Returns amount of jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Creates a copy of existing tour deeply copying all activities and jobs.
    pub fn deep_copy(&self) -> Tour {
        Tour {
            activities: self.activities.iter().map(|a| a.deep_copy()).collect(),
            jobs: self.jobs.clone(),
            is_closed: self.is_closed,
        }
    }
}

impl Debug for Tour {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("is_closed", &self.is_closed)
            .field("jobs", &self.jobs.len())
            .field(
                "activities",
                &self
                    .activities
                    .iter()
                    .enumerate()
                    .map(|(idx, activity)| match idx {
                        0 => "departure".to_string(),
                        idx if self.is_closed && idx == self.activities.len() - 1 => "arrival".to_string(),
                        _ => activity
                            .retrieve_job()
                            .and_then(|job| job.dimens().get_id().cloned())
                            .unwrap_or("undef".to_string()),
                    })
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Creates start activity.
fn create_start_activity(actor: &Actor) -> Activity {
    let start = &actor.detail.start.as_ref().unwrap_or_else(|| unimplemented!("{}", OP_START_MSG));
    let time = start.time.to_time_window();

    Activity {
        schedule: Schedule { arrival: time.start, departure: time.start },
        place: Place { location: start.location, duration: 0.0, time },
        job: None,
        commute: None,
    }
}

/// Creates end activity if it is specified for the actor.
fn create_end_activity(actor: &Actor) -> Option<Activity> {
    actor.detail.end.as_ref().map(|place| {
        let time = place.time.to_time_window();
        Activity {
            schedule: Schedule { arrival: time.start, departure: time.start },
            place: Place { location: place.location, duration: 0.0, time },
            job: None,
            commute: None,
        }
    })
}
