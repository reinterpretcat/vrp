use crate::construction::clustering::vicinity::*;
use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::helpers::models::problem::{get_job_id, SingleBuilder};
use crate::models::common::{Duration, IdDimension, Location, Profile, ValueDimension};
use crate::models::problem::Job;
use hashbrown::HashSet;
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;
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
        let constraints = vec![ConstraintVariant::HardRoute(Arc::new(VicinityHardRouteConstraint {
            disallow_merge_list: disallow_merge_list.clone(),
        }))];
        Self { disallow_merge_list, constraints, keys: Vec::default() }
    }
}

impl ConstraintModule for VicinityTestModule {
    fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {
        unimplemented!()
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, _: &mut SolutionContext) {
        unimplemented!()
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, i32> {
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

struct VicinityHardRouteConstraint {
    disallow_merge_list: HashSet<String>,
}

impl HardRouteConstraint for VicinityHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, _: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if self.disallow_merge_list.contains(job.dimens().get_id().unwrap()) {
            Some(RouteConstraintViolation { code: 1 })
        } else {
            None
        }
    }
}

pub fn create_constraint_pipeline(disallow_merge_list: Vec<&str>) -> ConstraintPipeline {
    let mut pipeline = ConstraintPipeline::default();

    let disallow_merge_list = disallow_merge_list.into_iter().map(|id| id.to_string()).collect();

    pipeline.add_module(Arc::new(VicinityTestModule::new(disallow_merge_list)));

    pipeline
}

pub fn create_cluster_config() -> ClusterConfig {
    let ordering_rule = |result: Ordering, left_job: &Job, right_job: &Job| match result {
        Ordering::Equal => get_job_id(left_job).cmp(get_job_id(right_job)),
        Ordering::Less => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
    };

    ClusterConfig {
        profile: Profile::new(0, None),
        threshold: ThresholdPolicy {
            moving_duration: 10.,
            moving_distance: 10.,
            min_shared_time: None,
            smallest_time_window: None,
            max_jobs_per_cluster: None,
        },
        visiting: VisitPolicy::Return,
        serving: ServingPolicy::Original { parking: 0. },
        filtering: FilterPolicy { job_filter: Arc::new(|_| true), actor_filter: Arc::new(|_| true) },
        building: BuilderPolicy {
            ordering_global: Arc::new(move |(left_job, left_candidates), (right_job, right_candidates)| {
                ordering_rule(left_candidates.len().cmp(&right_candidates.len()), left_job, right_job)
            }),
            ordering_local: Arc::new(move |left, right| {
                ordering_rule(
                    compare_floats(left.commute.forward.duration, right.commute.forward.duration),
                    &left.job,
                    &right.job,
                )
            }),
        },
    }
}
