use crate::helpers::ToLocation;
use crate::json::problem::*;

fn delivery_job_prototype() -> Job {
    Job {
        id: "job".to_owned(),
        places: JobPlaces {
            pickup: None,
            delivery: Some(JobPlace {
                times: Some(vec![vec![]]),
                location: vec![1., 0.].to_loc(),
                duration: 1.,
                tag: None,
            }),
        },
        demand: vec![1],
        skills: None,
    }
}
