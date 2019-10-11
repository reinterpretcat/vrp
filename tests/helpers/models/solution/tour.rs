use crate::construction::constraints::{Demand, DemandDimension};
use crate::helpers::models::problem::{test_single_job, test_single_job_with_simple_demand};
use crate::helpers::models::solution::*;
use crate::models::common::Location;
use crate::models::problem::Job;
use crate::models::solution::TourActivity;
use std::borrow::BorrowMut;
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

pub fn test_tour_activity_with_simple_demand(demand: Demand<i32>) -> TourActivity {
    let job = Arc::new(test_single_job_with_simple_demand(demand));
    Box::new(test_activity_with_job(job))
}
