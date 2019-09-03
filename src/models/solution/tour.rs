use std::collections::HashSet;
use std::sync::Arc;

use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Place};
use std::borrow::Borrow;
use std::io::empty;

pub struct Tour {
    /// Stores activities in the order the performed.
    activities: Vec<Arc<Activity>>,

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
    pub fn set_start(&mut self, activity: Arc<Activity>) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(self.activities.is_empty());
        self.activities.push(activity);

        self
    }

    /// Sets tour end.
    pub fn set_end(&mut self, activity: Arc<Activity>) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(!self.activities.is_empty());
        self.activities.push(activity);
        self.is_closed = true;

        self
    }

    /// Inserts activity within its job to the end of tour.
    pub fn insert_last(&mut self, activity: Arc<Activity>) -> &mut Tour {
        self.insert_at(activity, self.activity_count() + 1);
        self
    }

    /// Inserts activity within its job at specified index.
    pub fn insert_at(&mut self, activity: Arc<Activity>, index: usize) -> &mut Tour {
        assert!(activity.job.is_some());
        assert!(!self.activities.is_empty());

        self.jobs.insert(activity.retrieve_job().unwrap());
        self.activities.insert(index, activity);

        self
    }

    /// Removes job within its activities from the tour.
    pub fn remove(&mut self, job: &Arc<Job>) {
        self.activities.retain(|a| a.has_same_job(job))
    }

    /// Returns all activities in tour for specific job.
    pub fn activities<'a>(&'a self, job: Arc<Job>) -> impl Iterator<Item = Arc<Activity>> + 'a {
        self.activities
            .iter()
            .cloned()
            .filter(move |a| a.has_same_job(&job))
    }

    /// Returns counted tour legs.
    pub fn legs<'a>(&'a self) -> impl Iterator<Item = (&'a [Arc<Activity>], usize)> + 'a {
        self.activities.windows(2).zip(0..)
    }

    /// Returns all jobs.
    pub fn jobs<'a>(&'a self) -> impl Iterator<Item = Arc<Job>> + 'a {
        self.jobs.iter().cloned()
    }

    /// Returns activity by its index in tour.
    pub fn get(&self, index: usize) -> Option<&Arc<Activity>> {
        self.activities.get(index)
    }

    /// Returns start activity in tour.
    pub fn start(&self) -> Option<&Arc<Activity>> {
        self.activities.first()
    }

    /// Returns end activity in tour.
    pub fn end(&self) -> Option<&Arc<Activity>> {
        self.activities.last()
    }

    /// Returns index of first job occurrence in the tour.
    pub fn index(&self, job: &Arc<Job>) -> Option<usize> {
        self.activities
            .iter()
            .position(move |a| a.has_same_job(&job))
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
                    Arc::new(Activity {
                        place: Place {
                            location: a.place.location.clone(),
                            duration: a.place.duration.clone(),
                            time: a.place.time.clone(),
                        },
                        schedule: a.schedule.clone(),
                        job: a.job.clone(),
                    })
                })
                .collect(),
            jobs: Default::default(),
            is_closed: self.is_closed,
        }
    }
}
