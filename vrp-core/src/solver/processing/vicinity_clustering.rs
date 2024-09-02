#[cfg(test)]
#[path = "../../../tests/unit/solver/processing/vicinity_clustering_test.rs"]
mod vicinity_clustering_test;

use super::*;
use crate::construction::clustering::vicinity::*;
use crate::models::common::{Duration, Schedule};
use crate::models::problem::Jobs;
use crate::models::solution::{Activity, Place};
use crate::models::{Extras, GoalContext, Problem};
use crate::solver::RefinementContext;
use std::collections::HashSet;
use std::sync::Arc;

custom_extra_property!(ClusterConfig typeof ClusterConfig);
custom_extra_property!(OriginalProblem typeof Problem);

/// Provides a way to change problem definition by reducing total job count using clustering.
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

        let clusters = create_job_clusters(problem.clone(), environment, &config);

        if clusters.is_empty() {
            context
        } else {
            let (clusters, clustered_jobs) = clusters.into_iter().fold(
                (Vec::new(), HashSet::new()),
                |(mut clusters, mut clustered_jobs), (cluster, cluster_jobs)| {
                    clusters.push(cluster);
                    clustered_jobs.extend(cluster_jobs);

                    (clusters, clustered_jobs)
                },
            );

            let jobs = problem.jobs.all().filter(|job| !clustered_jobs.contains(job)).chain(clusters).collect();

            let mut extras: Extras = problem.extras.as_ref().clone();
            extras.set_original_problem(problem.clone());

            let problem = Arc::new(Problem {
                fleet: problem.fleet.clone(),
                jobs: Arc::new(Jobs::new(problem.fleet.as_ref(), jobs, problem.transport.as_ref())),
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
        let orig_problem = insertion_ctx.problem.extras.get_original_problem();

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
                        .and_then(|job| job.dimens().get_cluster_info().cloned())
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
                        let place_idx = 0;
                        let place = &job.places[place_idx];

                        let backward = match config.visiting {
                            VisitPolicy::Return => info.commute.backward.duration,
                            VisitPolicy::ClosedContinuation if info.job == last_job => info.commute.backward.duration,
                            _ => Duration::default(),
                        };

                        let service_time = info.service_time;
                        let service_start = (arrival + info.commute.forward.duration).max(cluster_time.start);
                        let departure = service_start + service_time + backward;

                        activities.push(Activity {
                            place: Place {
                                idx: place_idx,
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
                    .get_cluster_info()
                    .map(|clusters| clusters.iter().map(|info| (info.job.clone(), code.clone())).collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![(job.clone(), code.clone())])
                    .into_iter()
            })
            .collect();

        insertion_ctx.problem = orig_problem.clone();

        insertion_ctx
    }
}
