use crate::construction::constraints::{ConstraintModule, ConstraintPipeline, ConstraintVariant};
use crate::construction::heuristics::*;
use crate::helpers::models::problem::SingleBuilder;
use crate::models::common::{Duration, IdDimension, Location, ValueDimension};
use crate::models::problem::Job;
use hashbrown::HashSet;
use std::slice::Iter;
use std::sync::Arc;

pub const MERGED_KEY: &str = "merged";

pub type JobPlaces = Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>;

struct VicinityTestModule {
    disallow_merge_list: HashSet<String>,
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl VicinityTestModule {
    pub fn new(disallow_merge_list: HashSet<String>) -> Self {
        Self { disallow_merge_list, constraints: Vec::default(), keys: Vec::default() }
    }
}

impl ConstraintModule for VicinityTestModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {
        unimplemented!()
    }

    fn accept_route_state(&self, _: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, _: &mut SolutionContext) {
        unimplemented!()
    }

    fn merge_constrained(&self, source: Job, candidate: Job) -> Result<Job, i32> {
        if self.disallow_merge_list.contains(candidate.dimens().get_id().unwrap()) {
            Err(1)
        } else {
            let source = source.to_single();
            assert_eq!(source.places.len(), 1);

            let place = source.places.first().unwrap();
            let place = (
                place.location,
                place.duration,
                place
                    .times
                    .iter()
                    .map(|t| {
                        let tw = t.as_time_window().unwrap();
                        (tw.start, tw.end)
                    })
                    .collect::<Vec<_>>(),
            );

            let mut dimens = source.dimens.clone();
            let mut merged = dimens.get_value::<Vec<Job>>(MERGED_KEY).cloned().unwrap_or_else(Vec::new);
            merged.push(candidate);
            dimens.set_value(MERGED_KEY, merged);

            Ok(SingleBuilder::default().dimens(dimens).places(vec![place]).build_as_job_ref())
        }
    }

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

pub fn create_constraint_pipeline(disallow_merge_list: Vec<&str>) -> ConstraintPipeline {
    let mut pipeline = ConstraintPipeline::default();

    let disallow_merge_list = disallow_merge_list.into_iter().map(|id| id.to_string()).collect();

    pipeline.add_module(Arc::new(VicinityTestModule::new(disallow_merge_list)));

    pipeline
}
