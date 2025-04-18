//! Provides functionality to group jobs in some vicinity radius.

#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/vicinity/vicinity_test.rs"]
mod vicinity_test;

use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::models::common::Dimensions;
use crate::models::common::*;
use crate::models::problem::{Actor, Job};
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::sync::Arc;

mod estimations;
use self::estimations::*;
use crate::models::solution::Commute;
use crate::prelude::ViolationCode;

custom_dimension!(pub ClusterInfo typeof Vec<ClusterInfo>);

/// Holds center job and its neighbor jobs.
pub type ClusterCandidate<'a> = (&'a Job, &'a HashSet<Job>);

type CheckInsertionFn = (dyn Fn(&Job) -> Result<(), ViolationCode> + Send + Sync);

/// Specifies clustering algorithm configuration.
#[derive(Clone)]
pub struct ClusterConfig {
    /// A matrix profile used to calculate traveling durations and distances.
    pub profile: Profile,
    /// A thresholds for job clustering.
    pub threshold: ThresholdPolicy,
    /// Job visiting policy
    pub visiting: VisitPolicy,
    /// Job service time policy.
    pub serving: ServingPolicy,
    /// Specifies filtering policy.
    pub filtering: FilterPolicy,
    /// Specifies building policy.
    pub building: BuilderPolicy,
}

/// Defines a various thresholds to control cluster size.
#[derive(Clone)]
pub struct ThresholdPolicy {
    /// Moving duration limit.
    pub moving_duration: Duration,
    /// Moving distance limit.
    pub moving_distance: Distance,
    /// Minimum shared time for jobs (non-inclusive).
    pub min_shared_time: Option<Duration>,
    /// The smallest time window of the cluster after service time shrinking.
    pub smallest_time_window: Option<Duration>,
    /// The maximum amount of jobs per cluster.
    pub max_jobs_per_cluster: Option<usize>,
}

/// Specifies cluster visiting policy.
#[derive(Clone)]
pub enum VisitPolicy {
    /// It is required to return to the first job's location (cluster center) before visiting a next job.
    Return,
    /// Clustered jobs are visited one by one from the cluster center finishing in the end at the
    /// first job's location.
    ClosedContinuation,
    /// Clustered jobs are visited one by one starting from the cluster center and finishing in the
    /// end at the last job's location.
    /// NOTE: this might be useful for use clustering algorithm to split problem into sub-problems.
    /// TODO: make sure that it can be used with other non-clustered activities in the same stop.
    OpenContinuation,
}

/// Specifies filtering policy.
#[derive(Clone)]
pub struct FilterPolicy {
    /// Job filter.
    pub job_filter: Arc<dyn Fn(&Job) -> bool + Send + Sync>,
    /// Actor filter.
    pub actor_filter: Arc<dyn Fn(&Actor) -> bool + Send + Sync>,
}

/// Specifies service time policy.
#[derive(Clone)]
pub enum ServingPolicy {
    /// Keep original service time.
    Original {
        /// Parking time.
        parking: Duration,
    },
    /// Correct service time by some multiplier.
    Multiplier {
        /// Multiplier value applied to original job's duration.
        multiplier: Float,
        /// Parking time.
        parking: Duration,
    },
    /// Use fixed value for all clustered jobs.
    Fixed {
        /// Fixed value used for all jobs in the cluster.
        value: Duration,
        /// Parking time.
        parking: Duration,
    },
}

/// A function type which orders visiting clusters based on their estimated size.
pub type OrderingGlobalFn = Arc<dyn Fn(ClusterCandidate, ClusterCandidate) -> Ordering + Send + Sync>;
/// A function type which orders visiting jobs in a cluster based on their visit info.
pub type OrderingLocalFn = Arc<dyn Fn(&ClusterInfo, &ClusterInfo) -> Ordering + Send + Sync>;

/// Allows to control how clusters are built.
#[derive(Clone)]
pub struct BuilderPolicy {
    /// Orders visiting clusters based on their estimated size.
    pub ordering_global_fn: OrderingGlobalFn,
    /// Orders visiting jobs in a cluster based on their visit info.
    pub ordering_local_fn: OrderingLocalFn,
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
    /// Commute information.
    pub commute: Commute,
}

/// Creates clusters of jobs grouping them together best on vicinity properties.
/// Limitations:
/// - only single jobs are clustered
/// - time offset in job times is not supported
pub fn create_job_clusters(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    config: &ClusterConfig,
) -> Vec<(Job, Vec<Job>)> {
    let insertion_ctx = InsertionContext::new_empty(problem.clone(), environment);
    let constraint = insertion_ctx.problem.goal.clone();
    let check_insertion = get_check_insertion_fn(insertion_ctx, config.filtering.actor_filter.clone());
    let transport = problem.transport.as_ref();
    let jobs = problem
        .jobs
        .all()
        .iter()
        .filter(|job| (config.filtering.job_filter)(job))
        // NOTE multi-job is not supported
        .filter(|job| job.as_single().is_some())
        .cloned()
        .collect::<Vec<_>>();

    let estimates = get_jobs_dissimilarities(jobs.as_slice(), transport, config);

    get_clusters(&constraint, estimates, config, &check_insertion)
}

/// Gets function which checks possibility of cluster insertion.
fn get_check_insertion_fn(
    insertion_ctx: InsertionContext,
    actor_filter: Arc<dyn Fn(&Actor) -> bool + Send + Sync>,
) -> impl Fn(&Job) -> Result<(), ViolationCode> {
    move |job: &Job| -> Result<(), ViolationCode> {
        let eval_ctx = EvaluationContext {
            goal: &insertion_ctx.problem.goal,
            job,
            leg_selection: &LegSelection::Exhaustive,
            result_selector: &BestResultSelector::default(),
        };

        insertion_ctx
            .solution
            .registry
            .next_route()
            .filter(|route_ctx| (actor_filter)(&route_ctx.route().actor))
            .try_fold(Err(ViolationCode::unknown()), |_, route_ctx| {
                let result = eval_job_insertion_in_route(
                    &insertion_ctx,
                    &eval_ctx,
                    route_ctx,
                    InsertionPosition::Any,
                    InsertionResult::make_failure(),
                );

                match result {
                    InsertionResult::Success(_) => ControlFlow::Break(Ok(())),
                    InsertionResult::Failure(failure) => ControlFlow::Continue(Err(failure.constraint)),
                }
            })
            .unwrap_value()
    }
}

impl ServingPolicy {
    /// Gets parking time.
    pub fn get_parking(&self) -> Duration {
        match &self {
            Self::Original { parking } => *parking,
            Self::Multiplier { parking, .. } => *parking,
            Self::Fixed { parking, .. } => *parking,
        }
    }
}
