use super::*;
use crate::format::problem::*;
use crate::format::Location;
use std::ops::Range;
use uuid::Uuid;

/// Creates delivery job prototype.
pub fn delivery_job_prototype(
    task_proto: impl Strategy<Value = JobTask>,
    priority_proto: impl Strategy<Value = Option<i32>>,
    skills_proto: impl Strategy<Value = Option<JobSkills>>,
    value_proto: impl Strategy<Value = Option<f64>>,
) -> impl Strategy<Value = Job> {
    job_prototype(
        generate_no_job_tasks(),
        task_proto.prop_map(|p| Some(vec![p])),
        generate_no_job_tasks(),
        generate_no_job_tasks(),
        priority_proto,
        skills_proto,
        value_proto,
    )
}

/// Creates pickup job prototype.
pub fn pickup_job_prototype(
    task_proto: impl Strategy<Value = JobTask>,
    priority_proto: impl Strategy<Value = Option<i32>>,
    skills_proto: impl Strategy<Value = Option<JobSkills>>,
    value_proto: impl Strategy<Value = Option<f64>>,
) -> impl Strategy<Value = Job> {
    job_prototype(
        task_proto.prop_map(|p| Some(vec![p])),
        generate_no_job_tasks(),
        generate_no_job_tasks(),
        generate_no_job_tasks(),
        priority_proto,
        skills_proto,
        value_proto,
    )
}

prop_compose! {
    pub fn pickup_delivery_prototype(
        pickup_place: impl Strategy<Value = JobPlace>,
        delivery_place: impl Strategy<Value = JobPlace>,
        demand_proto: impl Strategy<Value = Option<Vec<i32>>>,
        priority_proto: impl Strategy<Value = Option<i32>>,
        skills_proto: impl Strategy<Value = Option<JobSkills>>,
        value_proto: impl Strategy<Value = Option<f64>>
    )
    (
     pickup in pickup_place,
     delivery in delivery_place,
     demand in demand_proto,
     priority in priority_proto,
     skills in skills_proto,
     value in value_proto,
    ) -> Job {
       Job {
            id: Uuid::new_v4().to_string(),
            pickups: Some(vec![
             JobTask { places: vec![pickup], demand: demand.clone(), tag: Some("p1".to_owned())}
            ]),
            deliveries: Some(vec![
             JobTask { places: vec![delivery], demand: demand.clone(), tag: Some("d1".to_owned())}
            ]),
            replacements: None,
            services: None,
            priority,
            skills,
            value
        }
    }
}

/// Generates jobs.
pub fn generate_jobs(job_proto: impl Strategy<Value = Job>, range: Range<usize>) -> impl Strategy<Value = Vec<Job>> {
    prop::collection::vec(job_proto, range)
}

/// Generates job plan.
pub fn generate_plan(jobs_proto: impl Strategy<Value = Vec<Job>>) -> impl Strategy<Value = Plan> {
    jobs_proto.prop_map(|jobs| Plan { jobs, relations: None })
}

prop_compose! {
   fn job_prototype(
        pickups_proto: impl Strategy<Value = Option<Vec<JobTask>>>,
        deliveries_proto: impl Strategy<Value = Option<Vec<JobTask>>>,
        replacements_proto: impl Strategy<Value = Option<Vec<JobTask>>>,
        services_proto: impl Strategy<Value = Option<Vec<JobTask>>>,
        priority_proto: impl Strategy<Value = Option<i32>>,
        skills_proto: impl Strategy<Value = Option<JobSkills>>,
        value_proto: impl Strategy<Value = Option<f64>>,
    )
    (
     pickups in pickups_proto,
     deliveries in deliveries_proto,
     replacements in replacements_proto,
     services in services_proto,
     priority in priority_proto,
     skills in skills_proto,
     value in value_proto
    ) -> Job {
        Job {
            id: Uuid::new_v4().to_string(),
            pickups,
            deliveries,
            replacements,
            services,
            priority,
            skills,
            value,
        }
    }
}

prop_compose! {
    pub fn job_task_prototype(
        places: impl Strategy<Value = JobPlace>,
        demand_proto: impl Strategy<Value = Option<Vec<i32>>>,
        tags: impl Strategy<Value = Option<String>>,
    )
    (
     place in places,
     demand in demand_proto,
     tag in tags
    ) -> JobTask {
       JobTask { places: vec![place], demand, tag}
    }
}

prop_compose! {
    pub fn job_place_prototype(
        locations: impl Strategy<Value = Location>,
        durations: impl Strategy<Value = f64>,
        time_windows: impl Strategy<Value = Option<Vec<Vec<String>>>>,
    )
    (
     location in locations,
     duration in durations,
     times in time_windows
    ) -> JobPlace {
      JobPlace { times, location, duration}
    }
}

prop_compose! {
    /// Generates one dimensional demand in range.
    pub fn generate_simple_demand(range: Range<i32>)(demand in range) -> Option<Vec<i32>> {
        Some(vec![demand])
    }
}

prop_compose! {
    /// Generates no tags.
    pub fn generate_no_tags()(_ in ".*") -> Option<String> {
        None
    }
}

prop_compose! {
    /// Generates no job place.
    pub fn generate_no_job_tasks()(_ in ".*") -> Option<Vec<JobTask>> {
        None
    }
}

prop_compose! {
    /// Generates no job priority.
    pub fn generate_no_priority()(_ in ".*") -> Option<i32> {
        None
    }
}

prop_compose! {
    /// Generates no job skills.
    pub fn generate_no_jobs_skills()(_ in ".*") -> Option<JobSkills> {
        None
    }
}

prop_compose! {
    /// Generates no job value.
    pub fn generate_no_jobs_value()(_ in ".*") -> Option<f64> {
        None
    }
}
