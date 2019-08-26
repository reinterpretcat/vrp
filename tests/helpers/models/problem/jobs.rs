use crate::models::common::Location;
use crate::models::problem::{Job, Multi, Place, Single};

pub fn test_single_job() -> Job {
    Job::Single(Single {
        places: Default::default(),
        dimens: Default::default(),
    })
}

pub fn test_place_with_location(location: Option<Location>) -> Place {
    Place {
        location,
        duration: Default::default(),
        times: Default::default(),
    }
}

pub fn test_single_job_with_location(location: Option<Location>) -> Job {
    Job::Single(Single {
        places: vec![test_place_with_location(location)],
        dimens: Default::default(),
    })
}

pub fn test_single_job_with_locations(locations: Vec<Option<Location>>) -> Job {
    Job::Single(Single {
        places: locations
            .into_iter()
            .map(|location| test_place_with_location(location))
            .collect(),
        dimens: Default::default(),
    })
}

pub fn test_multi_job_with_locations(locations: Vec<Vec<Option<Location>>>) -> Job {
    Job::Multi(Multi {
        jobs: locations
            .into_iter()
            .map(|locs| match test_single_job_with_locations(locs) {
                Job::Single(single) => single,
                _ => panic!("Unexpected job type!"),
            })
            .collect(),
        dimens: Default::default(),
    })
}
