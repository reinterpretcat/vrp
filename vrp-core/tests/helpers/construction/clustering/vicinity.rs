use crate::construction::clustering::vicinity::*;
use crate::construction::heuristics::*;
use crate::helpers::models::domain::GoalContextBuilder;
use crate::helpers::models::problem::{get_job_id, SingleBuilder};
use crate::models::common::{Duration, IdDimension, Location, Profile, ValueDimension};
use crate::models::problem::Job;
use crate::models::*;
use hashbrown::HashSet;
use rosomaxa::prelude::compare_floats;
use std::cmp::Ordering;
use std::sync::Arc;

pub const MERGED_KEY: &str = "merged";

pub type JobPlaces = Vec<(Option<Location>, Duration, Vec<(f64, f64)>)>;

struct VicinityTestFeatureConstraint {
    disallow_merge_list: HashSet<String>,
}

impl FeatureConstraint for VicinityTestFeatureConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { job, .. } => {
                if self.disallow_merge_list.contains(job.dimens().get_id().unwrap()) {
                    ConstraintViolation::fail(1)
                } else {
                    None
                }
            }
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
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
            let mut merged = dimens.get_value::<Vec<Job>>(MERGED_KEY).cloned().unwrap_or_default();
            merged.push(candidate);
            dimens.set_value(MERGED_KEY, merged);

            Ok(SingleBuilder::default().dimens(dimens).places(vec![place]).build_as_job_ref())
        }
    }
}

pub fn create_goal_context_with_vicinity(disallow_merge_list: Vec<&str>) -> GoalContext {
    let disallow_merge_list = disallow_merge_list.into_iter().map(|id| id.to_string()).collect();

    GoalContextBuilder::default()
        .add_feature(
            FeatureBuilder::default()
                .with_name("vicinity")
                .with_constraint(VicinityTestFeatureConstraint { disallow_merge_list })
                .build()
                .unwrap(),
        )
        .build()
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
            ordering_global_fn: Arc::new(move |(left_job, left_candidates), (right_job, right_candidates)| {
                ordering_rule(left_candidates.len().cmp(&right_candidates.len()), left_job, right_job)
            }),
            ordering_local_fn: Arc::new(move |left, right| {
                ordering_rule(
                    compare_floats(left.commute.forward.duration, right.commute.forward.duration),
                    &left.job,
                    &right.job,
                )
            }),
        },
    }
}
