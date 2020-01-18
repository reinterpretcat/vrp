use crate::construction::constraints::Demand;
use crate::helpers::models::problem::test_single_with_simple_demand;
use crate::helpers::models::solution::*;
use crate::models::common::{Duration, Location, Schedule};
use crate::models::problem::Single;
use crate::models::solution::TourActivity;
use std::sync::Arc;

pub fn test_tour_activity_with_location(location: Location) -> TourActivity {
    Box::new(test_activity_with_location(location))
}

pub fn test_tour_activity_with_location_and_duration(location: Location, duration: Duration) -> TourActivity {
    Box::new(test_activity_with_location_and_duration(location, duration))
}

pub fn test_tour_activity_with_schedule(schedule: Schedule) -> TourActivity {
    Box::new(test_activity_with_schedule(schedule))
}

pub fn test_tour_activity_with_default_job() -> TourActivity {
    Box::new(test_activity())
}

pub fn test_tour_activity_with_job(job: Arc<Single>) -> TourActivity {
    Box::new(test_activity_with_job(job))
}

pub fn test_tour_activity_without_job() -> TourActivity {
    Box::new(test_activity_without_job())
}

pub fn test_tour_activity_with_simple_demand(demand: Demand<i32>) -> TourActivity {
    Box::new(test_activity_with_job(test_single_with_simple_demand(demand)))
}
