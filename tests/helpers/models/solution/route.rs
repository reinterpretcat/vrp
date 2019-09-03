use crate::helpers::models::problem::*;
use crate::models::common::Schedule;
use crate::models::problem::{Job, Single};
use crate::models::solution::{Activity, Place};
use std::sync::Arc;

pub const DEFAULT_ACTIVITY_SCHEDULE: Schedule = Schedule {
    departure: 0.0,
    arrival: 0.0,
};

pub fn test_activity() -> Activity {
    Activity {
        place: Place {
            location: DEFAULT_JOB_LOCATION,
            duration: DEFAULT_JOB_DURATION,
            time: DEFAULT_JOB_TIME_WINDOW,
        },
        schedule: DEFAULT_ACTIVITY_SCHEDULE,
        job: Some(Arc::new(test_single_job())),
    }
}

pub struct ActivityBuilder {
    activity: Activity,
}

impl ActivityBuilder {
    pub fn new() -> ActivityBuilder {
        ActivityBuilder {
            activity: test_activity(),
        }
    }

    pub fn place(&mut self, place: Place) -> &mut ActivityBuilder {
        self.activity.place = place;
        self
    }

    pub fn schedule(&mut self, schedule: Schedule) -> &mut ActivityBuilder {
        self.activity.schedule = schedule;
        self
    }

    pub fn job(&mut self, job: Option<Arc<Job>>) -> &mut ActivityBuilder {
        self.activity.job = job;
        self
    }

    pub fn build(&mut self) -> Activity {
        std::mem::replace(&mut self.activity, test_activity())
    }
}
