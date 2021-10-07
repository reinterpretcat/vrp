use crate::models::common::*;
use crate::models::problem::{Actor, Job, Place, TransportCost};
use crate::models::Problem;
use crate::utils::compare_floats;
use std::ops::Deref;
use std::sync::Arc;

/// Specifies clustering algorithm configuration.
pub struct ClusterConfig {
    /// A thresholds for job clustering.
    threshold: ThresholdPolicy,
    /// Job visiting policy
    visiting: VisitPolicy,
    /// Job service time policy.
    service_time: ServiceTimePolicy,
    /// Specifies filtering policy.
    filtering: FilterPolicy,
}

/// Defines a various thresholds to control cluster size.
pub struct ThresholdPolicy {
    /// Moving duration limit.
    moving_duration: Duration,
    /// Moving distance limit.
    moving_distance: Distance,
    /// Minimum shared time for jobs.
    min_shared_time: Option<Duration>,
}

/// Specifies cluster visiting policy.
pub enum VisitPolicy {
    /// It is required to return to the first job's location (cluster center) before visiting a next job.
    Repetition,
    /// Clustered jobs are visited one by one from the cluster center finishing in the end at the
    /// first job's location.
    ClosedContinuation,
    /// Clustered jobs are visited one by one starting from the cluster center and finishing in the
    /// end at the last job's location.
    OpenContinuation,
}

/// Specifies filtering policy.
pub struct FilterPolicy {
    /// Job filter.
    job_filter: Arc<dyn Fn(&Job) -> bool + Send + Sync>,
    /// Actor filter.
    actor_filter: Arc<dyn Fn(&Actor) -> bool + Send + Sync>,
}

/// Specifies service time policy.
pub enum ServiceTimePolicy {
    /// Keep original service time.
    Original,
    /// Reduce service time by some multiplier.
    Reduced { multiplier: f64 },
    /// Use fixed value for all clustered jobs.
    Fixed,
}

/// Creates clusters of jobs trying to minimize their radius.
pub fn create_job_clusters(problem: &Problem, profile: &Profile, config: &ClusterConfig) -> Vec<Vec<Job>> {
    let transport = problem.transport.as_ref();
    let jobs = problem
        .jobs
        .all()
        .filter(&*config.filtering.job_filter)
        // NOTE multi-job is not supported
        .filter(|job| job.as_single().is_some())
        .collect::<Vec<_>>();
    let actors = problem
        .fleet
        .actors
        .iter()
        .filter(|actor| config.filtering.actor_filter.deref()(actor))
        .cloned()
        .collect::<Vec<_>>();

    let distances = jobs
        .iter()
        .map(|outer| {
            let dissimilarities = jobs
                .iter()
                .filter(|inner| outer != *inner)
                .filter_map(|inner| {
                    estimate_job_dissimilarities(&outer, inner, profile, &config.threshold, transport)
                        .map(|estimate| (inner.clone(), estimate.0, estimate.1))
                })
                .collect::<Vec<_>>();
            (outer.clone(), dissimilarities)
        })
        .collect::<Vec<_>>();

    unimplemented!()
}

fn estimate_job_dissimilarities(
    outer: &Job,
    inner: &Job,
    profile: &Profile,
    threshold: &ThresholdPolicy,
    transport: &(dyn TransportCost + Send + Sync),
) -> Option<(Distance, Duration)> {
    let departure = Default::default();
    outer
        .to_single()
        .places
        .iter()
        .filter_map(map_place)
        .flat_map(|(outer_loc, outer_times)| {
            inner.to_single().places.iter().filter_map(map_place).filter_map(move |(inner_loc, inner_times)| {
                let shared_time = outer_times
                    .iter()
                    .flat_map(|outer_time| {
                        inner_times
                            .iter()
                            .filter_map(move |inner_time| outer_time.overlapping(inner_time).map(|tw| tw.duration()))
                    })
                    .max_by(|a, b| compare_floats(*a, *b))
                    .unwrap_or(0.);

                if shared_time > threshold.min_shared_time.unwrap_or(0.) {
                    let distance = transport.distance(profile, outer_loc, inner_loc, departure);
                    let duration = transport.duration(profile, outer_loc, inner_loc, departure);

                    match ((duration - threshold.moving_duration < 0.), (distance - threshold.moving_distance < 0.)) {
                        (true, true) => Some((shared_time, distance, duration)),
                        _ => None,
                    }
                } else {
                    None
                }
            })
        })
        .max_by(|(left, _, _), (right, _, _)| compare_floats(*left, *right))
        .map(|(_, distance, duration)| (distance, duration))
}

fn build_job_cluster(
    cluster_estimate: &(Job, Vec<(Job, Distance, Duration)>),
    actors: &[Arc<Actor>],
    config: &ClusterConfig,
) -> Option<(Job, Vec<Job>)> {
    todo!()
}

fn map_place(place: &Place) -> Option<(Location, Vec<TimeWindow>)> {
    place
        .location
        .map(|location| (location, place.times.iter().filter_map(|time| time.as_time_window()).collect::<Vec<_>>()))
}
