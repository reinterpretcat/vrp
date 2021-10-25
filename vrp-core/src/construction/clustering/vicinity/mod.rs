//! Provides functionality to group jobs in some vicinity radius.

#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/vicinity/vicinity_test.rs"]
mod vicinity_test;

use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::common::{Dimensions, ValueDimension};
use crate::models::problem::{Actor, Job};
use crate::models::Problem;
use crate::utils::{unwrap_from_result, Environment};
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

mod estimations;
use self::estimations::*;

const CLUSTER_DIMENSION_KEY: &str = "cls";

/// A trait to get or set cluster info.
pub trait ClusterDimension {
    /// Sets cluster.
    fn set_cluster(&mut self, jobs: Vec<ClusterInfo>) -> &mut Self;
    /// Gets cluster.
    fn get_cluster(&self) -> Option<&Vec<ClusterInfo>>;
}

impl ClusterDimension for Dimensions {
    fn set_cluster(&mut self, jobs: Vec<ClusterInfo>) -> &mut Self {
        self.set_value(CLUSTER_DIMENSION_KEY, jobs);
        self
    }

    fn get_cluster(&self) -> Option<&Vec<ClusterInfo>> {
        self.get_value(CLUSTER_DIMENSION_KEY)
    }
}

/// Holds center job and its neighbor jobs.
pub type ClusterCandidate<'a> = (&'a Job, &'a HashSet<Job>);

type CheckInsertionFn = (dyn Fn(&Job) -> Result<(), i32> + Send + Sync);

/// Specifies clustering algorithm configuration.
pub struct ClusterConfig {
    /// A thresholds for job clustering.
    pub threshold: ThresholdPolicy,
    /// Job visiting policy
    pub visiting: VisitPolicy,
    /// Job service time policy.
    pub service_time: ServiceTimePolicy,
    /// Specifies filtering policy.
    pub filtering: FilterPolicy,
    /// Specifies building policy.
    pub building: BuilderPolicy,
}

/// Defines a various thresholds to control cluster size.
pub struct ThresholdPolicy {
    /// Moving duration limit.
    pub moving_duration: Duration,
    /// Moving distance limit.
    pub moving_distance: Distance,
    /// Minimum shared time for jobs (non-inclusive).
    pub min_shared_time: Option<Duration>,
}

/// Specifies cluster visiting policy.
pub enum VisitPolicy {
    /// It is required to return to the first job's location (cluster center) before visiting a next job.
    Return,
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
    pub job_filter: Arc<dyn Fn(&Job) -> bool + Send + Sync>,
    /// Actor filter.
    pub actor_filter: Arc<dyn Fn(&Actor) -> bool + Send + Sync>,
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
    /// The smallest time window of the cluster after service time shrinking.
    pub smallest_time_window: Option<f64>,
    /// Checks whether given cluster is already good to go, so clustering more jobs is not needed.
    pub threshold: Arc<dyn Fn(&Job) -> bool + Send + Sync>,
    /// Orders visiting clusters based on their estimated size.
    pub ordering_global: Arc<dyn Fn(ClusterCandidate, ClusterCandidate) -> Ordering + Send + Sync>,
    /// Orders visiting jobs in a cluster based on their visit info.
    pub ordering_local: Arc<dyn Fn(&ClusterInfo, &ClusterInfo) -> Ordering + Send + Sync>,
}

/// Keeps track of information specific for job in the cluster.
#[derive(Clone)]
pub struct ClusterInfo {
    /// An original job.
    pub job: Job,
    /// An activity's service time.
    pub service_time: Duration,
    /// An used place index.
    pub place_idx: usize,
    /// Movement info in forward direction.
    pub forward: (Distance, Duration),
    /// Movement info in backward direction.
    pub backward: (Distance, Duration),
}

/// Creates clusters of jobs grouping them together best on vicinity properties.
/// Limitations:
/// - only single jobs are clustered
/// - time offset in job times is not supported
pub fn create_job_clusters(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    profile: &Profile,
    config: &ClusterConfig,
) -> Vec<(Job, Vec<Job>)> {
    let insertion_ctx = InsertionContext::new_empty(problem.clone(), environment);
    let constraint = insertion_ctx.problem.constraint.clone();
    let check_job = get_check_insertion_fn(insertion_ctx, config.filtering.actor_filter.as_ref());
    let transport = problem.transport.as_ref();
    let jobs = problem
        .jobs
        .all()
        .filter(&*config.filtering.job_filter)
        // NOTE multi-job is not supported
        .filter(|job| job.as_single().is_some())
        .collect::<Vec<_>>();

    let estimates = get_jobs_dissimilarities(jobs.as_slice(), profile, transport, config);

    get_clusters(&constraint, estimates, config, &check_job)
}

/// Gets function which checks possibility of cluster insertion.
fn get_check_insertion_fn(
    insertion_ctx: InsertionContext,
    actor_filter: &(dyn Fn(&Actor) -> bool + Send + Sync),
) -> impl Fn(&Job) -> Result<(), i32> {
    let result_selector = BestResultSelector::default();
    let routes = insertion_ctx
        .solution
        .registry
        .next()
        .filter(|route_ctx| actor_filter.deref()(&route_ctx.route.actor))
        .collect::<Vec<_>>();

    move |job: &Job| -> Result<(), i32> {
        unwrap_from_result(routes.iter().try_fold(Err(-1), |_, route_ctx| {
            let result = evaluate_job_insertion_in_route(
                &insertion_ctx,
                route_ctx,
                job,
                InsertionPosition::Any,
                InsertionResult::make_failure(),
                &result_selector,
            );

            match result {
                InsertionResult::Success(_) => Err(Ok(())),
                InsertionResult::Failure(failure) => Ok(Err(failure.constraint)),
            }
        }))
    }
}
