use crate::models::common::*;
use crate::models::problem::{FixedJobPermutation, Job, Multi, Place, Single};
use std::sync::Arc;

pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_SPAN: TimeSpan = TimeSpan::Window(TimeWindow { start: 0., end: 1000. });
pub const DEFAULT_ACTIVITY_TIME_WINDOW: TimeWindow = TimeWindow { start: 0., end: 1000. };

pub fn test_place_with_location(location: Option<Location>) -> Place {
    Place { location, duration: DEFAULT_JOB_DURATION, times: vec![DEFAULT_JOB_TIME_SPAN] }
}

pub fn test_single() -> Single {
    let mut single =
        Single { places: vec![test_place_with_location(Some(DEFAULT_JOB_LOCATION))], dimens: Default::default() };
    single.dimens.set_id("single");
    single
}

pub fn test_single_with_simple_demand(demand: Demand<SingleDimLoad>) -> Arc<Single> {
    let mut single = test_single();
    single.dimens.set_demand(demand);
    Arc::new(single)
}

pub fn test_single_with_id(id: &str) -> Arc<Single> {
    let mut single = test_single();
    single.dimens.set_id(id);
    Arc::new(single)
}

pub fn test_single_with_location(location: Option<Location>) -> Arc<Single> {
    Arc::new(Single { places: vec![test_place_with_location(location)], dimens: Default::default() })
}

pub fn test_single_with_id_and_location(id: &str, location: Option<Location>) -> Arc<Single> {
    let mut single = Single { places: vec![test_place_with_location(location)], dimens: Default::default() };
    single.dimens.set_id(id);
    Arc::new(single)
}

pub fn test_single_with_locations(locations: Vec<Option<Location>>) -> Arc<Single> {
    Arc::new(Single {
        places: locations.into_iter().map(test_place_with_location).collect(),
        dimens: Default::default(),
    })
}

pub fn test_multi_with_id(id: &str, jobs: Vec<Arc<Single>>) -> Arc<Multi> {
    let mut dimens = Dimensions::new();
    dimens.set_id(id);

    Multi::new_shared(jobs, dimens)
}

pub fn test_multi_job_with_locations(locations: Vec<Vec<Option<Location>>>) -> Arc<Multi> {
    Multi::new_shared(locations.into_iter().map(test_single_with_locations).collect(), Default::default())
}

pub fn test_multi_with_permutations(id: &str, jobs: Vec<Arc<Single>>, permutations: Vec<Vec<usize>>) -> Arc<Multi> {
    let mut dimens = Dimensions::new();
    dimens.set_id(id);

    Multi::new_shared_with_permutator(jobs, dimens, Box::new(FixedJobPermutation::new(permutations)))
}

pub fn get_job_id(job: &Job) -> &String {
    job.dimens().get_id().unwrap()
}

pub struct SingleBuilder {
    single: Single,
}

impl Default for SingleBuilder {
    fn default() -> Self {
        Self { single: test_single() }
    }
}

impl SingleBuilder {
    pub fn id(&mut self, id: &str) -> &mut Self {
        self.single.dimens.set_value("id", id.to_string());
        self
    }

    pub fn dimens(&mut self, dimens: Dimensions) -> &mut Self {
        self.single.dimens = dimens;
        self
    }

    pub fn location(&mut self, loc: Option<Location>) -> &mut Self {
        self.single.places.first_mut().unwrap().location = loc;
        self
    }

    pub fn duration(&mut self, dur: Duration) -> &mut Self {
        self.single.places.first_mut().unwrap().duration = dur;
        self
    }

    pub fn times(&mut self, times: Vec<TimeWindow>) -> &mut Self {
        self.single.places.first_mut().unwrap().times = times.into_iter().map(TimeSpan::Window).collect();
        self
    }

    pub fn demand(&mut self, demand: Demand<SingleDimLoad>) -> &mut Self {
        self.single.dimens.set_demand(demand);
        self
    }

    pub fn places(&mut self, places: Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>) -> &mut Self {
        self.single.places = places
            .into_iter()
            .map(|p| Place {
                location: p.0,
                duration: p.1,
                times: p.2.into_iter().map(|(start, end)| TimeSpan::Window(TimeWindow::new(start, end))).collect(),
            })
            .collect();

        self
    }

    pub fn build(&mut self) -> Single {
        std::mem::replace(&mut self.single, test_single())
    }

    pub fn build_shared(&mut self) -> Arc<Single> {
        Arc::new(self.build())
    }

    pub fn build_as_job_ref(&mut self) -> Job {
        Job::Single(Arc::new(self.build()))
    }
}
