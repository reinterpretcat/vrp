#[cfg(test)]
#[path = "../../../tests/unit/models/solution/tour_test.rs"]
mod tour_test;

use crate::models::OP_START_MSG;
use crate::models::common::Schedule;
use crate::models::problem::{Actor, Job, JobIdDimension};
use crate::models::solution::{Activity, Place};
use crate::utils::{Either, short_type_name};
use rustc_hash::FxHasher;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::hash::BuildHasherDefault;
use std::iter::once;
use std::ops::Index;
use std::slice::{Iter, IterMut};
use std::sync::Arc;

/// A tour leg.
pub type Leg<'a> = (&'a [Activity], usize);

/// Stores a tour payload which can be shared until one of its copies is modified.
#[derive(Clone, Default)]
struct TourData {
    /// Stores activities in the order the performed.
    activities: Vec<Activity>,

    /// Stores jobs in the order of their activities added.
    jobs: HashSet<Job, BuildHasherDefault<FxHasher>>,
}

/// Represents a tour, a smart container for jobs with their associated activities.
#[derive(Default)]
pub struct Tour {
    data: Arc<TourData>,

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
        assert!(self.data.activities.is_empty());
        Arc::make_mut(&mut self.data).activities.push(activity);

        self
    }

    /// Sets tour end.
    pub fn set_end(&mut self, activity: Activity) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(!self.data.activities.is_empty());
        Arc::make_mut(&mut self.data).activities.push(activity);
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
        assert!(!self.data.activities.is_empty());

        let job = activity.retrieve_job().unwrap();
        let data = Arc::make_mut(&mut self.data);
        data.jobs.insert(job);
        data.activities.insert(index, activity);

        self
    }

    /// Removes job within its activities from the tour.
    pub fn remove(&mut self, job: &Job) -> bool {
        let data = Arc::make_mut(&mut self.data);
        data.activities.retain(|a| !a.has_same_job(job));
        data.jobs.remove(job)
    }

    /// Removes activity and its job from the tour.
    pub fn remove_activity_at(&mut self, idx: usize) -> Job {
        let job = self
            .data
            .activities
            .get(idx)
            .and_then(|a| a.retrieve_job())
            .expect("Attempt to remove activity without job from the tour!");
        self.remove(&job);

        job
    }

    /// Returns activities slice in specific range (all inclusive).
    pub fn activities_slice(&self, start: usize, end: usize) -> &[Activity] {
        &self.data.activities[start..=end]
    }

    /// Returns all activities in tour.
    pub fn all_activities(&self) -> Iter<'_, Activity> {
        self.data.activities.iter()
    }

    /// Returns all activities in tour as mutable.
    pub fn all_activities_mut(&mut self) -> IterMut<'_, Activity> {
        Arc::make_mut(&mut self.data).activities.iter_mut()
    }

    /// Returns all activities in tour for a specific job.
    pub fn job_activities<'a>(&'a self, job: &'a Job) -> impl Iterator<Item = &'a Activity> + 'a {
        self.data.activities.iter().filter(move |a| a.has_same_job(job))
    }

    /// Returns counted tour legs.
    pub fn legs(&self) -> impl Iterator<Item = Leg<'_>> + '_ + Clone {
        let activities = &self.data.activities;
        let last_index = if activities.is_empty() { 0 } else { activities.len() - 1 };

        let window_size = if activities.len() == 1 { 1 } else { 2 };
        let legs = activities.windows(window_size).zip(0_usize..);

        let is_open_tour_with_jobs = !self.is_closed && last_index > 0;

        if is_open_tour_with_jobs {
            Either::Left(legs.chain(once((&activities[last_index..], last_index))))
        } else {
            Either::Right(legs)
        }
    }

    /// Returns all jobs.
    pub fn jobs(&'_ self) -> impl Iterator<Item = &Job> + '_ {
        self.data.jobs.iter()
    }

    /// Returns activity by its index in tour.
    pub fn get(&self, index: usize) -> Option<&Activity> {
        self.data.activities.get(index)
    }

    /// Returns mutable activity by its index in tour.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Activity> {
        Arc::make_mut(&mut self.data).activities.get_mut(index)
    }

    /// Returns start activity in tour.
    pub fn start(&self) -> Option<&Activity> {
        self.data.activities.first()
    }

    /// Returns end activity in tour.
    pub fn end(&self) -> Option<&Activity> {
        self.data.activities.last()
    }

    /// Returns end activity in tour.
    pub fn end_idx(&self) -> Option<usize> {
        self.data.activities.len().checked_sub(1)
    }

    /// Checks whether job is present in tour
    pub fn contains(&self, job: &Job) -> bool {
        self.data.jobs.contains(job)
    }

    /// Returns index of first job occurrence in the tour.
    pub fn index(&self, job: &Job) -> Option<usize> {
        self.data.activities.iter().position(move |a| a.has_same_job(job))
    }

    /// Returns index of last job occurrence in the tour.
    pub fn index_last(&self, job: &Job) -> Option<usize> {
        self.data.activities.iter().rposition(move |a| a.has_same_job(job))
    }

    /// Checks whether job is present in tour.
    pub fn has_job(&self, job: &Job) -> bool {
        self.data.jobs.contains(job)
    }

    /// Checks whether tour has jobs.
    pub fn has_jobs(&self) -> bool {
        !self.data.jobs.is_empty()
    }

    /// Returns total amount of job activities.
    pub fn job_activity_count(&self) -> usize {
        if self.data.activities.is_empty() {
            0
        } else {
            self.data.activities.len() - (if self.is_closed { 2 } else { 1 })
        }
    }

    /// Returns amount of all activities in tour.
    pub fn total(&self) -> usize {
        self.data.activities.len()
    }

    /// Returns amount of jobs.
    pub fn job_count(&self) -> usize {
        self.data.jobs.len()
    }

    /// Creates an independent copy, cloning activities and jobs lazily on the first mutation.
    pub fn deep_copy(&self) -> Tour {
        Tour { data: self.data.clone(), is_closed: self.is_closed }
    }
}

impl Index<usize> for Tour {
    type Output = Activity;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data.activities[index]
    }
}

impl Debug for Tour {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("is_closed", &self.is_closed)
            .field("jobs", &self.data.jobs.len())
            .field(
                "activities",
                &self
                    .data
                    .activities
                    .iter()
                    .enumerate()
                    .map(|(idx, activity)| match idx {
                        0 => "departure".to_string(),
                        idx if self.is_closed && idx == self.data.activities.len() - 1 => "arrival".to_string(),
                        _ => activity
                            .retrieve_job()
                            .and_then(|job| job.dimens().get_job_id().cloned())
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
        place: Place { idx: 0, location: start.location, duration: 0., time },
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
            place: Place { idx: 0, location: place.location, duration: 0.0, time },
            job: None,
            commute: None,
        }
    })
}
