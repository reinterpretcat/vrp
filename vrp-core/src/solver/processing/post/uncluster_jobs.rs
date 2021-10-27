use super::*;
use crate::construction::clustering::vicinity::{ClusterConfig, ClusterDimension, VisitPolicy};
use crate::models::common::Schedule;
use crate::models::solution::{Activity, Place};
use crate::models::Problem;
use crate::solver::processing::ORIG_PROBLEM_KEY;

/// Unclusters previously clustered jobs in the solution.
pub struct UnclusterJobs {
    config: ClusterConfig,
}

impl PostProcessing for UnclusterJobs {
    fn process(&self, insertion_ctx: InsertionContext) -> InsertionContext {
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

        // TODO process unassigned jobs too

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
