use super::*;
use crate::construction::clustering::vicinity::*;
use crate::models::common::Schedule;
use crate::models::problem::Jobs;
use crate::models::solution::{Activity, Place};
use crate::models::{Extras, Problem};
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

const ORIG_PROBLEM_KEY: &str = "orig_problem";

/// Provides way to change problem definition by reducing total job count using clustering.
pub struct VicinityClustering {
    config: ClusterConfig,
}

impl VicinityClustering {
    /// Creates a new instance of `VicinityClustering`.
    pub fn new(config: ClusterConfig) -> Self {
        Self { config }
    }
}

impl Processing for VicinityClustering {
    fn pre_process(&self, problem: Arc<Problem>, environment: Arc<Environment>) -> Arc<Problem> {
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

            let mut extras: Extras =
                problem.extras.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<_, _>>();
            extras.insert(ORIG_PROBLEM_KEY.to_string(), problem.clone());

            Arc::new(Problem {
                fleet: problem.fleet.clone(),
                jobs: Arc::new(Jobs::new(problem.fleet.as_ref(), jobs, &problem.transport)),
                locks: problem.locks.clone(),
                constraint: problem.constraint.clone(),
                activity: problem.activity.clone(),
                transport: problem.transport.clone(),
                objective: problem.objective.clone(),
                extras: Arc::new(extras),
            })
        }
    }

    fn post_process(&self, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        insertion_ctx.solution.routes.iter_mut().for_each(|route_ctx| {
            let clusters = route_ctx
                .route
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

            clusters.into_iter().rev().for_each(|(cluster_idx, cluster)| {
                let cluster_activity = route_ctx.route.tour.get(cluster_idx).unwrap();
                let cluster_time = cluster_activity.place.time.clone();
                let cluster_arrival = cluster_activity.schedule.arrival;
                let last_job = cluster.last().unwrap().job.clone();

                let (_, activities) =
                    cluster.into_iter().fold((cluster_arrival, Vec::new()), |(arrival, mut activities), info| {
                        // NOTE assumption: no waiting time possible in between of clustered jobs

                        let job = info.job.to_single().clone();
                        let place = job.places.first().unwrap();

                        let movement = match self.config.visiting {
                            VisitPolicy::Return => info.forward.1 + info.backward.1,
                            VisitPolicy::OpenContinuation => info.forward.1,
                            VisitPolicy::ClosedContinuation => {
                                info.forward.1 + if info.job == last_job { info.backward.1 } else { 0. }
                            }
                        };
                        let departure = arrival.max(cluster_time.start) + movement + info.service_time;

                        activities.push(Activity {
                            place: Place {
                                location: place.location.unwrap(),
                                duration: info.service_time,
                                time: cluster_time.clone(),
                            },
                            schedule: Schedule::new(arrival, departure),
                            job: Some(job),
                        });

                        (departure, activities)
                    });

                route_ctx.route_mut().tour.remove_activity_at(cluster_idx);
                activities.into_iter().enumerate().for_each(|(seq_idx, activity)| {
                    route_ctx.route_mut().tour.insert_at(activity, cluster_idx + seq_idx);
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
                    .map(|clusters| clusters.iter().map(|info| (info.job.clone(), *code)).collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![(job.clone(), *code)])
                    .into_iter()
            })
            .collect();

        insertion_ctx.problem = insertion_ctx
            .problem
            .extras
            .get(ORIG_PROBLEM_KEY)
            .cloned()
            .and_then(|any| any.downcast::<Problem>().ok())
            .expect("no original problem ");

        insertion_ctx
    }
}
