use crate::models::common::{Duration, Location, Size, TimeWindow};
use crate::models::problem::{Job, Multi, Place, Single};
use std::any::Any;
use std::sync::Arc;

pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_WINDOW: TimeWindow = TimeWindow { start: 0.0, end: 1000.0 };

pub fn test_single() -> Single {
    let mut single = Single {
        places: vec![Place {
            location: Some(DEFAULT_JOB_LOCATION),
            duration: DEFAULT_JOB_DURATION,
            times: vec![DEFAULT_JOB_TIME_WINDOW],
        }],
        dimens: Default::default(),
    };
    single.dimens.insert("id".to_string(), Box::new("single".to_string()));
    single
}

pub fn test_single_job() -> Job {
    Job::Single(test_single())
}

pub fn test_place_with_location(location: Option<Location>) -> Place {
    Place { location, duration: DEFAULT_JOB_DURATION, times: vec![DEFAULT_JOB_TIME_WINDOW] }
}

pub fn test_single_job_with_location(location: Option<Location>) -> Job {
    Job::Single(Single { places: vec![test_place_with_location(location)], dimens: Default::default() })
}

pub fn test_single_job_with_locations(locations: Vec<Option<Location>>) -> Job {
    Job::Single(Single {
        places: locations.into_iter().map(|location| test_place_with_location(location)).collect(),
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

pub fn get_job_id(job: &Job) -> &String {
    match job {
        Job::Single(single) => &single.dimens,
        Job::Multi(multi) => &multi.dimens,
    }
    .get(&"id".to_string())
    .unwrap()
    .downcast_ref::<String>()
    .unwrap()
}

pub struct SingleBuilder {
    single: Single,
}

impl SingleBuilder {
    pub fn new() -> SingleBuilder {
        SingleBuilder { single: test_single() }
    }

    pub fn id(&mut self, id: &str) -> &mut SingleBuilder {
        self.single.dimens.insert("id".to_string(), Box::new(id.to_string()));
        self
    }

    pub fn demand(&mut self, demand: impl Size + 'static) -> &mut SingleBuilder {
        self.single.dimens.insert("dmd".to_string(), Box::new(demand));
        self
    }

    pub fn location(&mut self, loc: Option<Location>) -> &mut SingleBuilder {
        self.single.places.first_mut().unwrap().location = loc;
        self
    }

    pub fn duration(&mut self, dur: Duration) -> &mut SingleBuilder {
        self.single.places.first_mut().unwrap().duration = dur;
        self
    }

    pub fn time(&mut self, tw: TimeWindow) -> &mut SingleBuilder {
        let mut original_tw = self.single.places.first_mut().unwrap().times.first_mut().unwrap();
        original_tw.start = tw.start;
        original_tw.end = tw.end;

        self
    }

    pub fn build(&mut self) -> Single {
        std::mem::replace(&mut self.single, test_single())
    }

    pub fn build_as_job_ref(&mut self) -> Arc<Job> {
        let single = std::mem::replace(&mut self.single, test_single());
        Arc::new(Job::Single(single))
    }
}
