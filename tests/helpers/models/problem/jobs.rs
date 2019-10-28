use crate::construction::constraints::{CapacityDimension, Demand, DemandDimension};
use crate::models::common::{Duration, IdDimension, Location, TimeWindow};
use crate::models::problem::{Job, Multi, Place, Single};
use crate::models::Problem;
use std::sync::Arc;

pub const DEFAULT_JOB_LOCATION: Location = 0;
pub const DEFAULT_JOB_DURATION: Duration = 0.0;
pub const DEFAULT_JOB_TIME_WINDOW: TimeWindow = TimeWindow { start: 0.0, end: 1000.0 };

pub fn test_single() -> Single {
    test_single_with_id("single")
}

pub fn test_single_with_id(id: &str) -> Single {
    let mut single = Single {
        places: vec![Place {
            location: Some(DEFAULT_JOB_LOCATION),
            duration: DEFAULT_JOB_DURATION,
            times: vec![DEFAULT_JOB_TIME_WINDOW],
        }],
        dimens: Default::default(),
    };
    single.dimens.set_id(id);
    single
}

pub fn test_single_job() -> Job {
    Job::Single(Arc::new(test_single()))
}

pub fn test_single_job_with_simple_demand(demand: Demand<i32>) -> Job {
    let mut job = test_single();
    job.dimens.set_demand(demand);
    Job::Single(Arc::new(job))
}

pub fn test_place_with_location(location: Option<Location>) -> Place {
    Place { location, duration: DEFAULT_JOB_DURATION, times: vec![DEFAULT_JOB_TIME_WINDOW] }
}

pub fn test_single_job_with_id(id: &str) -> Job {
    let mut single =
        Single { places: vec![test_place_with_location(Some(DEFAULT_JOB_LOCATION))], dimens: Default::default() };
    single.dimens.set_id(id);
    Job::Single(Arc::new(single))
}

pub fn test_single_job_with_location(location: Option<Location>) -> Job {
    Job::Single(Arc::new(Single { places: vec![test_place_with_location(location)], dimens: Default::default() }))
}

pub fn test_single_job_with_id_and_location(id: &str, location: Option<Location>) -> Job {
    let mut single = Single { places: vec![test_place_with_location(location)], dimens: Default::default() };
    single.dimens.set_id(id);
    Job::Single(Arc::new(single))
}

pub fn test_single_job_with_locations(locations: Vec<Option<Location>>) -> Job {
    Job::Single(Arc::new(Single {
        places: locations.into_iter().map(|location| test_place_with_location(location)).collect(),
        dimens: Default::default(),
    }))
}

pub fn test_multi_job_with_locations(locations: Vec<Vec<Option<Location>>>) -> Job {
    Job::Multi(Multi::bind(Multi::new(
        locations
            .into_iter()
            .map(|locs| match test_single_job_with_locations(locs) {
                Job::Single(single) => single.clone(),
                _ => panic!("Unexpected job type!"),
            })
            .collect(),
        Default::default(),
    )))
}

pub fn get_job_id(job: &Job) -> &String {
    match job {
        Job::Single(single) => &single.dimens,
        Job::Multi(multi) => &multi.dimens,
    }
    .get_id()
    .unwrap()
}

pub fn get_job_simple_demand(job: &Job) -> &Demand<i32> {
    match job {
        Job::Single(single) => &single.dimens,
        Job::Multi(multi) => &multi.dimens,
    }
    .get_demand()
    .unwrap()
}

pub struct SingleBuilder {
    single: Single,
}

impl SingleBuilder {
    pub fn new() -> Self {
        Self { single: test_single() }
    }

    pub fn id(&mut self, id: &str) -> &mut Self {
        self.single.dimens.insert("id".to_string(), Box::new(id.to_string()));
        self
    }

    pub fn demand_capacity(&mut self, demand: usize) -> &mut Self {
        self.single.dimens.insert("dmd".to_string(), Box::new(demand));
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

    pub fn time(&mut self, tw: TimeWindow) -> &mut Self {
        let mut original_tw = self.single.places.first_mut().unwrap().times.first_mut().unwrap();
        original_tw.start = tw.start;
        original_tw.end = tw.end;

        self
    }

    pub fn times(&mut self, tws: Vec<TimeWindow>) -> &mut Self {
        self.single.places.first_mut().unwrap().times = tws;
        self
    }

    pub fn build(&mut self) -> Single {
        std::mem::replace(&mut self.single, test_single())
    }

    pub fn build_as_job_ref(&mut self) -> Arc<Job> {
        Arc::new(Job::Single(Arc::new(self.build())))
    }
}

fn test_multi() -> Multi {
    let mut multi = Multi::new(
        vec![Arc::new(test_single_with_id("single1")), Arc::new(test_single_with_id("single2"))],
        Default::default(),
    );
    multi.dimens.set_id("multi");
    multi
}

pub struct MultiBuilder {
    multi: Multi,
}

impl MultiBuilder {
    pub fn new() -> Self {
        let mut multi = Multi::new(vec![], Default::default());
        multi.dimens.set_id("multi");

        Self { multi }
    }

    pub fn new_with_permutations(permutations: Vec<Vec<usize>>) -> Self {
        let mut multi = Multi::new_with_generator(vec![], Default::default(), Box::new(move |_| permutations.clone()));
        multi.dimens.set_id("multi");

        Self { multi }
    }

    pub fn id(&mut self, id: &str) -> &mut Self {
        self.multi.dimens.set_id(id);
        self
    }

    pub fn job(&mut self, job: Single) -> &mut Self {
        self.multi.jobs.push(Arc::new(job));
        self
    }

    pub fn build(&mut self) -> Arc<Job> {
        let multi = std::mem::replace(&mut self.multi, test_multi());
        let multi = Multi::bind(multi);
        Arc::new(Job::Multi(multi))
    }
}

pub fn get_job_ids(problem: &Problem) -> Vec<String> {
    problem.jobs.all().map(|j| get_job_id(j.as_ref()).to_owned()).collect()
}

pub fn get_job_demands(problem: &Problem) -> Vec<i32> {
    problem.jobs.all().map(|j| get_job_simple_demand(j.as_ref()).delivery.0).collect()
}

pub fn get_vehicle_capacity(problem: &Problem) -> i32 {
    *problem.fleet.vehicles.iter().next().unwrap().dimens.get_capacity().unwrap()
}

pub fn get_job_time_windows(problem: &Problem) -> Vec<(f64, f64)> {
    problem
        .jobs
        .all()
        .map(|j| match j.as_ref() {
            Job::Single(j) => j.places.first().unwrap().times.first().map(|tw| (tw.start, tw.end)).unwrap(),
            _ => panic!(),
        })
        .collect()
}

pub fn get_job_durations(problem: &Problem) -> Vec<f64> {
    problem
        .jobs
        .all()
        .map(|j| match j.as_ref() {
            Job::Single(j) => j.places.first().unwrap().duration,
            _ => panic!(),
        })
        .collect()
}
