#[cfg(test)]
#[path = "../../../tests/unit/models/solution/tour_test.rs"]
mod tour_test;

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Place};
use std::borrow::Borrow;
use std::io::empty;
use std::slice::Iter;

pub type TourActivity = Arc<RwLock<Activity>>;

/// Represents a tour, a smart container for jobs with their associated activities.
pub struct Tour {
    /// Stores activities in the order the performed.
    activities: Vec<TourActivity>,

    /// Stores jobs in the order of their activities added.
    jobs: HashSet<Arc<Job>>,

    /// Keeps track whether tour is set as closed.
    is_closed: bool,
}

pub struct Statistic {}

impl Tour {
    pub fn new() -> Tour {
        Tour {
            activities: Default::default(),
            jobs: Default::default(),
            is_closed: false,
        }
    }

    /// Sets tour start.
    pub fn set_start(&mut self, activity: TourActivity) -> &mut Tour {
        assert!(activity.read().unwrap().job.is_none());
        assert!(self.activities.is_empty());
        self.activities.push(activity);

        self
    }

    /// Sets tour end.
    pub fn set_end(&mut self, activity: TourActivity) -> &mut Tour {
        assert!(activity.read().unwrap().job.is_none());
        assert!(!self.activities.is_empty());
        self.activities.push(activity);
        self.is_closed = true;

        self
    }

    /// Inserts activity within its job to the end of tour.
    pub fn insert_last(&mut self, activity: TourActivity) -> &mut Tour {
        self.insert_at(activity, self.activity_count() + 1);
        self
    }

    /// Inserts activity within its job at specified index.
    pub fn insert_at(&mut self, activity: TourActivity, index: usize) -> &mut Tour {
        {
            let activity = activity.read().unwrap();
            assert!(activity.job.is_some());
            assert!(!self.activities.is_empty());

            self.jobs.insert(activity.retrieve_job().unwrap());
        }
        self.activities.insert(index, activity);

        self
    }

    /// Removes job within its activities from the tour.
    pub fn remove(&mut self, job: &Arc<Job>) -> bool {
        self.activities
            .retain(|a| !a.read().unwrap().has_same_job(job));
        self.jobs.remove(job)
    }

    /// Returns all activities in tour.
    pub fn all_activities(&self) -> Iter<TourActivity> {
        self.activities.iter()
    }

    /// Returns all activities in tour for specific job.
    pub fn job_activities<'a>(
        &'a self,
        job: &'a Arc<Job>,
    ) -> impl Iterator<Item = TourActivity> + 'a {
        self.activities
            .iter()
            .filter(move |a| a.read().unwrap().has_same_job(job))
            .cloned()
    }

    /// Returns counted tour legs.
    pub fn legs<'a>(&'a self) -> impl Iterator<Item = (&'a [TourActivity], usize)> + 'a {
        self.activities.windows(2).zip(0..)
    }

    /// Returns all jobs.
    pub fn jobs<'a>(&'a self) -> impl Iterator<Item = Arc<Job>> + 'a {
        self.jobs.iter().cloned()
    }

    /// Returns activity by its index in tour.
    pub fn get(&self, index: usize) -> Option<&TourActivity> {
        self.activities.get(index)
    }

    /// Returns start activity in tour.
    pub fn start(&self) -> Option<&TourActivity> {
        self.activities.first()
    }

    /// Returns end activity in tour.
    pub fn end(&self) -> Option<&TourActivity> {
        self.activities.last()
    }

    /// Returns index of first job occurrence in the tour.
    pub fn index(&self, job: &Arc<Job>) -> Option<usize> {
        self.activities
            .iter()
            .position(move |a| a.read().unwrap().has_same_job(&job))
    }

    /// Checks whether tour is empty.
    pub fn empty(&self) -> bool {
        self.activities.is_empty()
    }

    /// Returns amount of job activities.
    pub fn activity_count(&self) -> usize {
        if self.empty() {
            0
        } else {
            self.activities.len() - (if self.is_closed { 2 } else { 1 })
        }
    }

    /// Returns amount of jobs.
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// Creates a copy of existing tour deeply copying all activities.
    pub fn copy(&self) -> Tour {
        Tour {
            activities: self
                .activities
                .iter()
                .map(|a| {
                    let a = a.read().unwrap();
                    Arc::new(RwLock::new(Activity {
                        place: Place {
                            location: a.place.location.clone(),
                            duration: a.place.duration.clone(),
                            time: a.place.time.clone(),
                        },
                        schedule: a.schedule.clone(),
                        job: a.job.clone(),
                    }))
                })
                .collect(),
            jobs: Default::default(),
            is_closed: self.is_closed,
        }
    }
}
