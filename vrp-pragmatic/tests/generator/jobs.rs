use super::*;
use crate::json::problem::*;
use crate::json::Location;
use std::ops::Range;
use uuid::Uuid;

prop_compose! {
    /// Generates durations in range.
    pub fn generate_durations(range: Range<i32>)(duration in range) -> f64 {
        duration as f64
    }
}

prop_compose! {
    /// Generates one dimensional demand in range.
    pub fn generate_simple_demand(range: Range<i32>)(demand in range) -> Vec<i32> {
        vec![demand]
    }
}

prop_compose! {
    /// Generates no tags.
    pub fn generate_no_tags()(_ in ".*") -> Option<String> {
        None
    }
}

prop_compose! {
    /// Generates no skills.
    pub fn generate_no_skills()(_ in ".*") -> Option<Vec<String>> {
        None
    }
}

prop_compose! {
    /// Generates no job place.
    pub fn generate_no_simple_job_place()(_ in ".*") -> Option<JobPlace> {
        None
    }
}

/// Creates delivery job prototype.
pub fn delivery_job_prototype(
    delivery_proto: impl Strategy<Value = JobPlace>,
    demand_proto: impl Strategy<Value = Vec<i32>>,
    skills_proto: impl Strategy<Value = Option<Vec<String>>>,
) -> impl Strategy<Value = Job> {
    simple_job_prototype(
        generate_no_simple_job_place(),
        delivery_proto.prop_map(|p| Some(p)),
        demand_proto,
        skills_proto,
    )
}

/// Creates pickup job prototype.
pub fn pickup_job_prototype(
    pickup_proto: impl Strategy<Value = JobPlace>,
    demand_proto: impl Strategy<Value = Vec<i32>>,
    skills_proto: impl Strategy<Value = Option<Vec<String>>>,
) -> impl Strategy<Value = Job> {
    simple_job_prototype(pickup_proto.prop_map(|p| Some(p)), generate_no_simple_job_place(), demand_proto, skills_proto)
}

/// Creates pickup and delivery job prototype.
pub fn pickup_delivery_job_prototype(
    pickup_proto: impl Strategy<Value = JobPlace>,
    delivery_proto: impl Strategy<Value = JobPlace>,
    demand_proto: impl Strategy<Value = Vec<i32>>,
    skills_proto: impl Strategy<Value = Option<Vec<String>>>,
) -> impl Strategy<Value = Job> {
    simple_job_prototype(
        pickup_proto.prop_map(|p| Some(p)),
        delivery_proto.prop_map(|p| Some(p)),
        demand_proto,
        skills_proto,
    )
}

/// Generates jobs.
pub fn generate_jobs(
    job_proto: impl Strategy<Value = JobVariant>,
    range: Range<usize>,
) -> impl Strategy<Value = Vec<JobVariant>> {
    prop::collection::vec(job_proto, range)
}

/// Generates job plan.
pub fn generate_plan(jobs_proto: impl Strategy<Value = Vec<JobVariant>>) -> impl Strategy<Value = Plan> {
    jobs_proto.prop_map(|jobs| Plan { jobs, relations: None })
}

prop_compose! {
    fn simple_job_prototype(
        pickup_proto: impl Strategy<Value = Option<JobPlace>>,
        delivery_proto: impl Strategy<Value = Option<JobPlace>>,
        demand_proto: impl Strategy<Value = Vec<i32>>,
        skills_proto: impl Strategy<Value = Option<Vec<String>>>,
    )
    (pickup in pickup_proto,
     delivery in delivery_proto,
     demand in demand_proto,
     skills in skills_proto) -> Job {
        Job {
            id: Uuid::new_v4().to_string(),
            places: JobPlaces {
                pickup,
                delivery,
            },
            demand,
            skills,
        }
    }
}

prop_compose! {
    pub fn simple_job_place_prototype(
        locations: impl Strategy<Value = Location>,
        durations: impl Strategy<Value = f64>,
        tags: impl Strategy<Value = Option<String>>,
        time_windows: impl Strategy<Value = Vec<Vec<String>>>,
    )
    (location in locations,
     duration in durations,
     tag in tags,
     times in time_windows) -> JobPlace {
      JobPlace {
        times: Some(times),
        location,
        duration,
        tag,
      }
    }
}
