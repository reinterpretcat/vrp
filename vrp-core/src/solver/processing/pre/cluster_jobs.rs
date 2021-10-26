use super::*;
use crate::construction::clustering::vicinity::{create_job_clusters, ClusterConfig};
use crate::models::problem::Jobs;
use crate::models::Problem;
use hashbrown::HashSet;
use std::sync::Arc;

/// Provides way to change problem definition by reducing total job count using clustering,
pub struct ClusterJobs {
    config: ClusterConfig,
}

impl ClusterJobs {
    /// Creates a new instance of `ClusterJobs`.
    pub fn new(config: ClusterConfig) -> Self {
        Self { config }
    }
}

impl PreProcessing for ClusterJobs {
    fn process(&self, problem: Arc<Problem>, environment: Arc<Environment>) -> Arc<Problem> {
        let clusters = create_job_clusters(problem.clone(), environment, &self.config);

        if clusters.is_empty() {
            problem
        } else {
            let (clusters, clustered_jobs) = clusters.into_iter().fold(
                (Vec::new(), HashSet::new()),
                |(mut clusters, mut clustered_jobs), (cluster, cluster_jobs)| {
                    clusters.push(cluster);
                    clustered_jobs.extend(cluster_jobs.into_iter());

                    (clusters, clustered_jobs)
                },
            );

            let jobs =
                problem.jobs.all().filter(|job| clustered_jobs.contains(job)).chain(clusters.into_iter()).collect();

            // TODO store info about clusters in extras?

            Arc::new(Problem {
                fleet: problem.fleet.clone(),
                jobs: Arc::new(Jobs::new(problem.fleet.as_ref(), jobs, &problem.transport)),
                locks: problem.locks.clone(),
                constraint: problem.constraint.clone(),
                activity: problem.activity.clone(),
                transport: problem.transport.clone(),
                objective: problem.objective.clone(),
                extras: problem.extras.clone(),
            })
        }
    }
}
