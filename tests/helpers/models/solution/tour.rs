use crate::helpers::models::solution::{
    test_activity, test_activity_with_job, test_activity_without_job,
};
use crate::models::problem::Job;
use crate::models::solution::{Activity, TourActivity};
use std::sync::{Arc, RwLock};

pub fn test_tour_activity_with_default_job() -> TourActivity {
    Arc::new(RwLock::new(test_activity()))
}

pub fn test_tour_activity_with_job(job: Arc<Job>) -> TourActivity {
    Arc::new(RwLock::new(test_activity_with_job(job)))
}

pub fn test_tour_activity_without_job() -> TourActivity {
    Arc::new(RwLock::new(test_activity_without_job()))
}
