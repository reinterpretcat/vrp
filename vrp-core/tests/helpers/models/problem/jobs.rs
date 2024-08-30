use crate::construction::features::JobDemandDimension;
use crate::models::common::*;
use crate::models::problem::{FixedJobPermutation, Job, JobIdDimension, Multi, Place, Single};
use rosomaxa::prelude::Float;
use std::sync::Arc;

pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_SPAN: TimeSpan = TimeSpan::Window(TimeWindow { start: 0., end: 1000. });
pub const DEFAULT_ACTIVITY_TIME_WINDOW: TimeWindow = TimeWindow { start: 0., end: 1000. };

pub type TestPlace = (Option<Location>, Duration, Vec<(Float, Float)>);

pub fn test_multi_with_id(id: &str, jobs: Vec<Arc<Single>>) -> Arc<Multi> {
    let mut dimens = Dimensions::default();
    dimens.set_job_id(id.to_string());

    Multi::new_shared(jobs, dimens)
}

pub fn test_multi_job_with_locations(locations: Vec<Vec<Option<Location>>>) -> Arc<Multi> {
    let jobs =
        locations.into_iter().map(|locations| TestSingleBuilder::with_locations(locations).build_shared()).collect();
    Multi::new_shared(jobs, Default::default())
}

pub fn test_multi_with_permutations(id: &str, jobs: Vec<Arc<Single>>, permutations: Vec<Vec<usize>>) -> Arc<Multi> {
    let mut dimens = Dimensions::default();
    dimens.set_job_id(id.to_string());

    Multi::new_shared_with_permutator(jobs, dimens, Box::new(FixedJobPermutation::new(permutations)))
}

pub fn get_job_id(job: &Job) -> &String {
    job.dimens().get_job_id().expect("no job id")
}

pub struct TestSingleBuilder(Single);

impl Default for TestSingleBuilder {
    fn default() -> Self {
        Self(test_single())
    }
}

impl TestSingleBuilder {
    pub fn with_locations(locations: Vec<Option<Location>>) -> Self {
        let mut single = Single {
            places: locations.into_iter().map(test_place_with_location).collect(),
            dimens: Default::default(),
        };
        single.dimens.set_job_id("single".to_string());

        Self(single)
    }

    pub fn id(&mut self, id: &str) -> &mut Self {
        self.0.dimens.set_job_id(id.to_string());
        self
    }

    pub fn property<K: 'static, T: 'static + Sync + Send>(&mut self, value: T) -> &mut Self {
        self.0.dimens.set_value::<K, _>(value);
        self
    }

    pub fn dimens(&mut self, dimens: Dimensions) -> &mut Self {
        self.0.dimens = dimens;
        self
    }

    pub fn location(&mut self, loc: Option<Location>) -> &mut Self {
        self.0.places.first_mut().unwrap().location = loc;
        self
    }

    pub fn duration(&mut self, dur: Duration) -> &mut Self {
        self.0.places.first_mut().unwrap().duration = dur;
        self
    }

    pub fn times(&mut self, times: Vec<TimeWindow>) -> &mut Self {
        self.0.places.first_mut().unwrap().times = times.into_iter().map(TimeSpan::Window).collect();
        self
    }

    pub fn demand<T: LoadOps>(&mut self, demand: Demand<T>) -> &mut Self {
        self.0.dimens.set_job_demand(demand);
        self
    }

    pub fn places(&mut self, places: Vec<TestPlace>) -> &mut Self {
        self.0.places = places
            .into_iter()
            .map(|p| Place {
                location: p.0,
                duration: p.1,
                times: p.2.into_iter().map(|(start, end)| TimeSpan::Window(TimeWindow::new(start, end))).collect(),
            })
            .collect();

        self
    }

    pub fn dimens_mut(&mut self) -> &mut Dimensions {
        &mut self.0.dimens
    }

    pub fn build(&mut self) -> Single {
        std::mem::replace(&mut self.0, test_single())
    }

    pub fn build_shared(&mut self) -> Arc<Single> {
        Arc::new(self.build())
    }

    pub fn build_as_job_ref(&mut self) -> Job {
        Job::Single(Arc::new(self.build()))
    }
}

fn test_single() -> Single {
    let mut single =
        Single { places: vec![test_place_with_location(Some(DEFAULT_JOB_LOCATION))], dimens: Default::default() };
    single.dimens.set_job_id("single".to_string());
    single
}

fn test_place_with_location(location: Option<Location>) -> Place {
    Place { location, duration: DEFAULT_JOB_DURATION, times: vec![DEFAULT_JOB_TIME_SPAN] }
}
