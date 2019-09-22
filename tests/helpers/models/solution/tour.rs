use crate::helpers::models::solution::{
    test_activity, test_activity_with_job, test_activity_with_location, test_activity_without_job,
};
use crate::models::common::Location;
use crate::models::problem::Job;
use crate::models::solution::TourActivity;
use std::sync::Arc;

pub fn test_tour_activity_with_location(location: Location) -> TourActivity {
    Box::new(test_activity_with_location(location))
}

pub fn test_tour_activity_with_default_job() -> TourActivity {
    Box::new(test_activity())
}

pub fn test_tour_activity_with_job(job: Arc<Job>) -> TourActivity {
    Box::new(test_activity_with_job(job))
}

pub fn test_tour_activity_without_job() -> TourActivity {
    Box::new(test_activity_without_job())
}
