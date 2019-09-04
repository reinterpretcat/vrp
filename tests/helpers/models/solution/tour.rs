use crate::helpers::models::solution::{
    test_activity, test_activity_with_job, test_activity_without_job,
};
use crate::models::problem::Job;
use crate::models::solution::Activity;
use std::sync::Arc;

pub fn test_tour_activity_with_default_job() -> Arc<Activity> {
    Arc::new(test_activity())
}

pub fn test_tour_activity_with_job(job: Arc<Job>) -> Arc<Activity> {
    Arc::new(test_activity_with_job(job))
}

pub fn test_tour_activity_without_job() -> Arc<Activity> {
    Arc::new(test_activity_without_job())
}
