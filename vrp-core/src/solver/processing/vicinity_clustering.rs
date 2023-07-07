#[cfg(test)]
#[path = "../../../tests/unit/solver/processing/vicinity_clustering_test.rs"]
mod vicinity_clustering_test;

use super::*;
use crate::construction::clustering::vicinity::*;
use crate::models::common::{Schedule, ValueDimension};
use crate::models::problem::Jobs;
use crate::models::solution::{Activity, Place};
use crate::models::{Extras, GoalContext, Problem};
use crate::solver::RefinementContext;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

const ORIG_PROBLEM_KEY: &str = "orig_problem";

/// A trait to get or set vicinity config.
pub trait VicinityDimension {
    /// Sets cluster config.
    fn set_cluster_config(&mut self, config: ClusterConfig) -> &mut Self;
    /// Gets cluster config.
    fn get_cluster_config(&self) -> Option<&ClusterConfig>;
}

impl VicinityDimension for Extras {
    fn set_cluster_config(&mut self, config: ClusterConfig) -> &mut Self {
        self.set_value("vicinity", config);
        self
    }

    fn get_cluster_config(&self) -> Option<&ClusterConfig> {
        self.get_value("vicinity")
    }
}

/// Provides way to change problem definition by reducing total job count using clustering.
#[derive(Default)]
pub struct VicinityClustering {}

impl HeuristicContextProcessing for VicinityClustering {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn pre_process(&self, context: Self::Context) -> Self::Context {
        let problem = context.problem.clone();
        let environment = context.environment.clone();

        let config = if let Some(config) = problem.extras.get_cluster_config() { config } else { return context };

        let clusters = create_job_clusters(problem.clone(), environment, config);

        if clusters.is_empty() {
            context
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
                problem.jobs.all().filter(|job| !clustered_jobs.contains(job)).chain(clusters.into_iter()).collect();

            let mut extras: Extras =
                problem.extras.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<_, _, _>>();
            extras.insert(ORIG_PROBLEM_KEY.to_string(), problem.clone());

            let problem = Arc::new(Problem {
                fleet: problem.fleet.clone(),
                jobs: Arc::new(Jobs::new(problem.fleet.as_ref(), jobs, &problem.transport)),
                locks: problem.locks.clone(),
                goal: problem.goal.clone(),
                activity: problem.activity.clone(),
                transport: problem.transport.clone(),
                extras: Arc::new(extras),
            });

            RefinementContext { problem, ..context }
        }
    }
}

impl HeuristicSolutionProcessing for VicinityClustering {
    type Solution = InsertionContext;

    fn post_process(&self, solution: Self::Solution) -> Self::Solution {
        let mut insertion_ctx = solution;

        let config = insertion_ctx.problem.extras.get_cluster_config();
        let orig_problem =
            insertion_ctx.problem.extras.get(ORIG_PROBLEM_KEY).cloned().and_then(|any| any.downcast::<Problem>().ok());

        let (config, orig_problem) = if let Some((config, orig_problem)) = config.zip(orig_problem) {
            (config, orig_problem)
        } else {
            return insertion_ctx;
        };

        insertion_ctx.solution.routes.iter_mut().for_each(|route_ctx| {
            #[allow(clippy::needless_collect)]
            let clusters = route_ctx
                .route()
                .tour
                .all_activities()
                .enumerate()
                .filter_map(|(idx, activity)| {
                    activity
                        .retrieve_job()
                        .and_then(|job| job.dimens().get_cluster().cloned())
                        .map(|cluster| (idx, cluster))
                })
                .collect::<Vec<_>>();

            clusters.into_iter().rev().for_each(|(activity_idx, cluster)| {
                let cluster_activity = route_ctx.route().tour.get(activity_idx).unwrap();
                let cluster_time = cluster_activity.place.time.clone();
                let cluster_arrival = cluster_activity.schedule.arrival;
                let last_job = cluster.last().unwrap().job.clone();

                let (_, activities) =
                    cluster.into_iter().fold((cluster_arrival, Vec::new()), |(arrival, mut activities), info| {
                        // NOTE assumption: no waiting time possible in between of clustered jobs
                        let job = info.job.to_single().clone();
                        let place = job.places.first().unwrap();

                        let backward = match config.visiting {
                            VisitPolicy::Return => info.commute.backward.duration,
                            VisitPolicy::ClosedContinuation if info.job == last_job => info.commute.backward.duration,
                            _ => 0.,
                        };

                        let service_time = info.service_time;
                        let service_start = (arrival + info.commute.forward.duration).max(cluster_time.start);
                        let departure = service_start + service_time + backward;

                        activities.push(Activity {
                            place: Place {
                                location: place.location.unwrap(),
                                duration: info.service_time,
                                time: cluster_time.clone(),
                            },
                            schedule: Schedule::new(arrival, departure),
                            job: Some(job),
                            commute: Some(info.commute),
                        });

                        (departure, activities)
                    });

                route_ctx.route_mut().tour.remove_activity_at(activity_idx);
                activities.into_iter().enumerate().for_each(|(seq_idx, activity)| {
                    route_ctx.route_mut().tour.insert_at(activity, activity_idx + seq_idx);
                });
            });
        });

        insertion_ctx.solution.unassigned = insertion_ctx
            .solution
            .unassigned
            .iter()
            .flat_map(|(job, code)| {
                job.dimens()
                    .get_cluster()
                    .map(|clusters| clusters.iter().map(|info| (info.job.clone(), code.clone())).collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![(job.clone(), code.clone())])
                    .into_iter()
            })
            .collect();

        insertion_ctx.problem = orig_problem;

        insertion_ctx
    }
}
