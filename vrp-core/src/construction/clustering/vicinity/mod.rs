//! Provides functionality to group jobs in some vicinity radius.

use crate::construction::heuristics::InsertionContext;
use crate::models::common::*;
use crate::models::common::{Dimensions, ValueDimension};
use crate::models::problem::{Actor, Job};
use crate::models::Problem;
use crate::utils::Environment;
use std::cmp::Ordering;
use std::sync::Arc;

mod estimations;
use self::estimations::*;

const CLUSTER_DIMENSION_KEY: &str = "cls";

/// A trait to get or set cluster info.
pub trait ClusterDimension {
    /// Sets cluster.
    fn set_cluster(&mut self, jobs: Vec<(Job, VisitInfo)>) -> &mut Self;
    /// Gets cluster.
    fn get_cluster(&self) -> Option<&Vec<(Job, VisitInfo)>>;
}

impl ClusterDimension for Dimensions {
    fn set_cluster(&mut self, jobs: Vec<(Job, VisitInfo)>) -> &mut Self {
        self.set_value(CLUSTER_DIMENSION_KEY, jobs);
        self
    }

    fn get_cluster(&self) -> Option<&Vec<(Job, VisitInfo)>> {
        self.get_value(CLUSTER_DIMENSION_KEY)
    }
}

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
    /// Specifies building policy.
    building: BuilderPolicy,
}

/// Defines a various thresholds to control cluster size.
pub struct ThresholdPolicy {
    /// Moving duration limit.
    moving_duration: Duration,
    /// Moving distance limit.
    moving_distance: Distance,
    /// Minimum shared time for jobs (non-inclusive).
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
    /// Correct service time by some multiplier.
    Multiplier(f64),
    /// Use fixed value for all clustered jobs.
    Fixed(f64),
}

/// Allows to control how clusters are built.
pub struct BuilderPolicy {
    /// The smallest time window of the cluster.
    smallest_time_window: Option<f64>,
    /// Checks whether given cluster is already good to go, so clustering more jobs is not needed.
    threshold: Arc<dyn Fn(&Job) -> bool + Send + Sync>,
    /// Orders visiting jobs based on their visit info.
    ordering: Arc<dyn Fn(&VisitInfo, &VisitInfo) -> Ordering + Send + Sync>,
}

/// Keeps track of information specific for job visiting.
#[derive(Clone)]
pub struct VisitInfo {
    /// An activity's service time.
    service_time: Duration,
    /// Movement info in forward direction.
    forward: (Distance, Duration),
    /// Movement info in backward direction.
    backward: (Distance, Duration),
}

/// Creates clusters of jobs grouping them together best on vicinity properties.
/// Limitations:
/// - only single jobs are clustered
/// - time offset is not supported
pub fn create_job_clusters(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    profile: &Profile,
    config: &ClusterConfig,
) -> Vec<(Job, Vec<Job>)> {
    let insertion_ctx = InsertionContext::new_empty(problem.clone(), environment);
    let constraint = insertion_ctx.problem.constraint.clone();
    let check_job = get_check_insertion_fn(insertion_ctx, config.filtering.actor_filter.as_ref());
    let estimates = get_estimates(problem.as_ref(), profile, config);

    get_clusters(&constraint, estimates, config, &check_job)
}
