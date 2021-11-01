use crate::core::construction::clustering::vicinity::*;
use crate::core::models::common::IdDimension;
use crate::core::models::problem::Job;
use crate::format::problem::reader::fleet_reader::get_profile_index_map;
use crate::format::problem::reader::ApiProblem;
use crate::format::problem::{Clustering, VehicleProfile, VicinityFilteringPolicy};
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::sync::Arc;
use vrp_core::construction::clustering::vicinity::ClusterConfig;
use vrp_core::models::common::Profile;
use vrp_core::utils::compare_floats;

pub(crate) fn create_cluster_config(api_problem: &ApiProblem) -> Result<Option<ClusterConfig>, String> {
    if let Some(clustering) = api_problem.plan.clustering.as_ref() {
        match clustering {
            Clustering::Vicinity { profile, threshold, visiting, serving, filtering } => Ok(Some(ClusterConfig {
                profile: get_profile(api_problem, profile)?,
                threshold: ThresholdPolicy {
                    moving_duration: threshold.moving_distance,
                    moving_distance: threshold.moving_duration,
                    min_shared_time: threshold.min_shared_time.clone(),
                    smallest_time_window: threshold.smallest_time_window.clone(),
                },
                visiting: VisitPolicy::Return,
                serving: ServingPolicy::Original,
                filtering: get_filter_policy(filtering.as_ref()),
                building: get_builder_policy(threshold.max_jobs_per_cluster.clone()),
            })),
        }
    } else {
        Ok(None)
    }
}

fn get_profile(api_problem: &ApiProblem, profile: &VehicleProfile) -> Result<Profile, String> {
    let profile_map = get_profile_index_map(api_problem);
    let profile_index = profile_map
        .get(&profile.matrix)
        .cloned()
        .ok_or_else(|| format!("cannot find matrix profile: {}", profile.matrix))?;

    Ok(Profile { index: profile_index, scale: profile.scale.unwrap_or(1.) })
}

fn get_builder_policy(max_jobs_per_cluster: Option<usize>) -> BuilderPolicy {
    // NOTE use ordering rule which is based on job id to make clusters stable
    let ordering_rule = |result: Ordering, left_job: &Job, right_job: &Job| match result {
        Ordering::Equal => match (left_job.dimens().get_id(), right_job.dimens().get_id()) {
            (Some(left), Some(right)) => left.cmp(right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
        Ordering::Less => Ordering::Less,
        Ordering::Greater => Ordering::Greater,
    };

    BuilderPolicy {
        threshold: if let Some(max_jobs_per_cluster) = max_jobs_per_cluster {
            Arc::new(move |job| {
                job.dimens().get_cluster().map_or(true, |clustered| clustered.len() < max_jobs_per_cluster)
            })
        } else {
            Arc::new(|_| true)
        },
        ordering_global: Arc::new(move |(left_job, left_candidates), (right_job, right_candidates)| {
            ordering_rule(left_candidates.len().cmp(&right_candidates.len()), left_job, right_job)
        }),
        ordering_local: Arc::new(move |left, right| {
            ordering_rule(compare_floats(left.forward.1, right.forward.1), &left.job, &right.job)
        }),
    }
}

fn get_filter_policy(filtering: Option<&VicinityFilteringPolicy>) -> FilterPolicy {
    if let Some(filtering) = filtering {
        let excluded_job_ids = filtering.exclude_job_ids.iter().cloned().collect::<HashSet<_>>();
        FilterPolicy {
            job_filter: Arc::new(move |job| {
                job.dimens().get_id().map_or(true, |job_id| !excluded_job_ids.contains(job_id))
            }),
            actor_filter: Arc::new(|_| true),
        }
    } else {
        FilterPolicy { job_filter: Arc::new(|_| true), actor_filter: Arc::new(|_| true) }
    }
}
