use std::collections::HashSet;
use std::sync::Arc;

use crate::models::problem::{Job, Single};
use crate::models::solution::Activity;
use std::borrow::Borrow;

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
        unimplemented!()
    }

    pub fn start(&mut self, activity: Arc<Activity>) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(self.activities.is_empty());
        self.activities.push(activity);

        self
    }

    pub fn end(&mut self, activity: Arc<Activity>) -> &mut Tour {
        assert!(activity.job.is_none());
        assert!(!self.activities.is_empty());
        self.activities.push(activity);
        self.is_closed = true;

        self
    }

    pub fn insert_last(&mut self) -> &mut Tour {
        unimplemented!();
        self
    }

    pub fn insert_at(&mut self, index: usize) -> &mut Tour {
        unimplemented!();
        self
    }

    pub fn remove(&mut self, job: &Arc<Job>) {
        self.activities.retain(|a| match a.retrieve_job() {
            Some(j) => j != *job,
            _ => true,
        })
    }
}
