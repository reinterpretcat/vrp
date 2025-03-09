use super::*;
use crate::format::problem::fleet_reader::get_profile_index_map;
use std::cmp::Ordering;
use std::collections::HashSet;
use vrp_core::construction::clustering::vicinity::*;
use vrp_core::models::common::Profile;
use vrp_core::models::problem::JobIdDimension;

/// Creates cluster config if it is defined on the api problem.
pub(super) fn create_cluster_config(api_problem: &ApiProblem) -> Result<Option<ClusterConfig>, GenericError> {
    if let Some(clustering) = api_problem.plan.clustering.as_ref() {
        match clustering {
            Clustering::Vicinity { profile, threshold, visiting, serving, filtering } => Ok(Some(ClusterConfig {
                profile: get_profile(api_problem, profile)?,
                threshold: ThresholdPolicy {
                    moving_duration: threshold.distance,
                    moving_distance: threshold.duration,
                    min_shared_time: threshold.min_shared_time,
                    smallest_time_window: threshold.smallest_time_window,
                    max_jobs_per_cluster: threshold.max_jobs_per_cluster,
                },
                visiting: match visiting {
                    VicinityVisitPolicy::Continue => VisitPolicy::ClosedContinuation,
                    VicinityVisitPolicy::Return => VisitPolicy::Return,
                },
                serving: match *serving {
                    VicinityServingPolicy::Original { parking } => ServingPolicy::Original { parking },
                    VicinityServingPolicy::Multiplier { value, parking } => {
                        ServingPolicy::Multiplier { multiplier: value, parking }
                    }
                    VicinityServingPolicy::Fixed { value, parking } => ServingPolicy::Fixed { value, parking },
                },
                filtering: get_filter_policy(api_problem, filtering.as_ref()),
                building: get_builder_policy(),
            })),
        }
    } else {
        Ok(None)
    }
}

fn get_profile(api_problem: &ApiProblem, profile: &VehicleProfile) -> Result<Profile, GenericError> {
    let profile_map = get_profile_index_map(api_problem);
    let profile_index = profile_map
        .get(&profile.matrix)
        .cloned()
        .ok_or_else(|| format!("cannot find matrix profile: {}", profile.matrix))?;

    Ok(Profile { index: profile_index, scale: profile.scale.unwrap_or(1.) })
}

fn get_builder_policy() -> BuilderPolicy {
    // NOTE use ordering rule which is based on job id to make clusters stable
    let ordering_rule = |result: Ordering, left_job: &CoreJob, right_job: &CoreJob| match result {
        Ordering::Equal => match (left_job.dimens().get_job_id(), right_job.dimens().get_job_id()) {
            (Some(left), Some(right)) => left.cmp(right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
        Ordering::Less => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
    };

    BuilderPolicy {
        ordering_global_fn: Arc::new(move |(left_job, left_candidates), (right_job, right_candidates)| {
            ordering_rule(left_candidates.len().cmp(&right_candidates.len()), left_job, right_job)
        }),
        ordering_local_fn: Arc::new(move |left, right| {
            ordering_rule(
                left.commute.forward.duration.total_cmp(&right.commute.forward.duration),
                &left.job,
                &right.job,
            )
        }),
    }
}

fn get_filter_policy(api_problem: &ApiProblem, filtering: Option<&VicinityFilteringPolicy>) -> FilterPolicy {
    let relation_ids = api_problem
        .plan
        .relations
        .iter()
        .flat_map(|relations| relations.iter())
        .flat_map(|relation| relation.jobs.iter())
        .cloned()
        .collect::<HashSet<_>>();

    let excluded_job_ids = if let Some(filtering) = filtering {
        filtering.exclude_job_ids.iter().cloned().chain(relation_ids).collect::<HashSet<_>>()
    } else {
        relation_ids
    };

    FilterPolicy {
        job_filter: Arc::new(move |job| {
            job.dimens().get_job_id().is_none_or(|job_id| !excluded_job_ids.contains(job_id))
        }),
        actor_filter: Arc::new(|_| true),
    }
}
