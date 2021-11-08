#[cfg(test)]
#[path = "../../../../tests/unit/construction/clustering/vicinity/estimations_test.rs"]
mod estimations_test;

use super::*;
use crate::construction::constraints::ConstraintPipeline;
use crate::models::common::*;
use crate::models::problem::{Place, Single, TransportCost};
use crate::models::solution::CommuteInfo;
use crate::utils::*;
use hashbrown::{HashMap, HashSet};
use std::ops::Deref;

type PlaceInfo = (PlaceIndex, Location, Duration, Vec<TimeWindow>);
type PlaceIndex = usize;
type Reachable = bool;
type DissimilarityInfo = (Reachable, PlaceIndex, ClusterInfo);
type DissimilarityIndex = HashMap<Job, Vec<DissimilarityInfo>>;

/// Gets job clusters.
pub(crate) fn get_clusters(
    constraint: &ConstraintPipeline,
    estimates: HashMap<Job, DissimilarityIndex>,
    config: &ClusterConfig,
    check_insertion: &CheckInsertionFn,
) -> Vec<(Job, Vec<Job>)> {
    let mut used_jobs = HashSet::new();
    let mut clusters = Vec::new();
    let mut cluster_estimates = estimates
        .iter()
        .map(|(job, estimate)| {
            let candidates = estimate
                .iter()
                .filter_map(|(job, infos)| {
                    // get only reachable estimates
                    let infos = infos.iter().filter(|(reachable, ..)| *reachable).collect::<Vec<_>>();
                    if infos.is_empty() {
                        None
                    } else {
                        Some(job.clone())
                    }
                })
                .collect::<HashSet<_>>();

            (job.clone(), (None, candidates))
        })
        .collect::<Vec<(_, (Option<Job>, HashSet<_>))>>();

    loop {
        parallel_foreach_mut(cluster_estimates.as_mut_slice(), |(center_job, (cluster, _))| {
            if cluster.is_none() {
                *cluster = build_job_cluster(constraint, center_job, &estimates, &used_jobs, config, check_insertion)
            }
        });

        cluster_estimates.sort_by(|(a_job, (_, a_can)), (b_job, (_, b_can))| {
            config.building.ordering_global.deref()((b_job, b_can), (a_job, a_can))
        });

        let new_cluster = cluster_estimates.first().and_then(|(_, (cluster, _))| cluster.as_ref()).cloned();

        if let Some(new_cluster) = new_cluster {
            let new_cluster_jobs = new_cluster
                .dimens()
                .get_cluster()
                .expect("expected to have jobs in a cluster")
                .iter()
                .map(|info| info.job.clone())
                .collect::<Vec<_>>();

            clusters.push((new_cluster.clone(), new_cluster_jobs.clone()));
            used_jobs.extend(new_cluster_jobs.into_iter());

            // remove used jobs from analysis
            cluster_estimates.retain(|(center, _)| !used_jobs.contains(center));
            cluster_estimates.iter_mut().for_each(|(_, (cluster, candidates))| {
                candidates.retain(|job| !used_jobs.contains(job));

                let is_cluster_affected = cluster
                    .as_ref()
                    .and_then(|cluster| cluster.dimens().get_cluster())
                    .map_or(false, |cluster_jobs| cluster_jobs.iter().any(|info| used_jobs.contains(&info.job)));

                if is_cluster_affected {
                    // NOTE force to rebuild cluster on next iteration
                    *cluster = None;
                }
            });
            cluster_estimates.retain(|(_, (_, candidates))| !candidates.is_empty());
        } else {
            break;
        }
    }

    clusters
}

/// Gets jobs dissimilarities.
pub(crate) fn get_jobs_dissimilarities(
    jobs: &[Job],
    transport: &(dyn TransportCost + Send + Sync),
    config: &ClusterConfig,
) -> HashMap<Job, DissimilarityIndex> {
    jobs.iter()
        .map(|outer| {
            let dissimilarities = jobs
                .iter()
                .filter(|inner| outer != *inner)
                .filter_map(|inner| {
                    let dissimilarities = get_dissimilarities(outer, inner, transport, config);
                    if dissimilarities.is_empty() {
                        None
                    } else {
                        Some((inner.clone(), dissimilarities))
                    }
                })
                .collect::<HashMap<_, _>>();
            (outer.clone(), dissimilarities)
        })
        .collect::<HashMap<_, _>>()
}

fn get_dissimilarities(
    outer: &Job,
    inner: &Job,
    transport: &(dyn TransportCost + Send + Sync),
    config: &ClusterConfig,
) -> Vec<DissimilarityInfo> {
    outer
        .to_single()
        .places
        .iter()
        .enumerate()
        .filter_map(map_place)
        .flat_map(|(outer_place_idx, outer_loc, _, outer_times)| {
            inner.to_single().places.iter().enumerate().filter_map(map_place).filter_map(
                move |(inner_place_idx, inner_loc, inner_duration, inner_times)| {
                    let shared_time = outer_times
                        .iter()
                        .flat_map(|outer_time| {
                            inner_times.iter().filter_map(move |inner_time| {
                                outer_time.overlapping(inner_time).map(|tw| tw.duration())
                            })
                        })
                        .max_by(|a, b| compare_floats(*a, *b))
                        .unwrap_or(0.);

                    if shared_time > config.threshold.min_shared_time.unwrap_or(0.) {
                        let departure = Default::default();

                        let fwd_distance = transport.distance(&config.profile, outer_loc, inner_loc, departure);
                        let fwd_duration = transport.duration(&config.profile, outer_loc, inner_loc, departure);

                        let bck_distance = transport.distance(&config.profile, inner_loc, outer_loc, departure);
                        let bck_duration = transport.duration(&config.profile, inner_loc, outer_loc, departure);

                        let reachable = compare_floats(fwd_distance, 0.) != Ordering::Less
                            && compare_floats(bck_distance, 0.) != Ordering::Less;

                        let reachable = reachable
                            && (fwd_duration - config.threshold.moving_duration < 0.)
                            && (fwd_distance - config.threshold.moving_distance < 0.)
                            && (bck_duration - config.threshold.moving_duration < 0.)
                            && (bck_distance - config.threshold.moving_distance < 0.);

                        let service_time = get_service_time(inner_duration, &config.serving);

                        let info = ClusterInfo {
                            job: inner.clone(),
                            service_time,
                            place_idx: inner_place_idx,
                            commute: Commute {
                                forward: CommuteInfo {
                                    location: outer_loc,
                                    distance: fwd_distance,
                                    duration: fwd_duration,
                                },
                                backward: CommuteInfo {
                                    location: outer_loc,
                                    distance: bck_distance,
                                    duration: bck_duration,
                                },
                            },
                        };

                        Some((reachable, outer_place_idx, info))
                    } else {
                        None
                    }
                },
            )
        })
        .collect()
}

fn build_job_cluster(
    constraint: &ConstraintPipeline,
    center_job: &Job,
    estimates: &HashMap<Job, DissimilarityIndex>,
    used_jobs: &HashSet<Job>,
    config: &ClusterConfig,
    check_insertion: &CheckInsertionFn,
) -> Option<Job> {
    let ordering = config.building.ordering_local.as_ref();
    let center = center_job.to_single();
    let center_estimates = estimates.get(center_job).expect("missing job in estimates");

    // iterate through all places and choose the one with most jobs clustered
    unwrap_from_result(center.places.iter().enumerate().filter_map(map_place).try_fold(
        Option::<(Job, usize)>::None,
        |best_cluster, center_place_info| {
            let (center_place_idx, center_location, center_duration, center_times) = center_place_info;
            let new_duration = get_service_time(center_duration, &config.serving);
            let new_center_job = create_single_job(Some(center_location), new_duration, &center_times, &center.dimens);
            let new_visit_info = ClusterInfo {
                job: center_job.clone(),
                service_time: new_duration,
                place_idx: center_place_idx,
                commute: Commute {
                    forward: CommuteInfo { location: center_place_idx, distance: 0., duration: 0. },
                    backward: CommuteInfo { location: center_place_idx, distance: 0., duration: 0. },
                },
            };
            let center_commute = |original_info: &ClusterInfo| {
                estimates
                    .get(center_job)
                    .and_then(|index| index.get(&original_info.job))
                    .and_then(|infos| {
                        infos.iter().find(|(_, outer_place_idx, info)| {
                            *outer_place_idx == center_place_idx && info.place_idx == original_info.place_idx
                        })
                    })
                    .map(|(_, _, info)| info.commute.clone())
                    .expect("cannot find movement info")
            };

            let is_max_jobs = |count| config.threshold.max_jobs_per_cluster.map_or(false, |max| max <= count);

            // allow jobs only from reachable candidates
            let mut cluster_candidates = center_estimates
                .iter()
                .filter(|(job, ..)| !used_jobs.contains(job))
                .filter(|(_, infos)| infos.iter().any(|(reachable, ..)| *reachable))
                .map(|(candidate, _)| candidate.clone())
                .collect::<HashSet<_>>();

            let mut cluster = with_cluster_dimension(new_center_job, new_visit_info);
            let mut last_job = center_job.clone();
            let mut last_place_idx = center_place_idx;
            let mut count = 1_usize;

            loop {
                if cluster_candidates.is_empty() || is_max_jobs(count) {
                    break;
                }

                // get job estimates specific for the last visited place
                let mut job_estimates = estimates
                    .get(&last_job)
                    .iter()
                    .flat_map(|index| index.iter().filter(|(job, _)| cluster_candidates.contains(job)))
                    .flat_map(|estimate| {
                        // embed the first visit info to sort estimates of all candidate jobs later
                        // we allow unreachable from the last job candidates as they must be reachable from the center
                        let include_unreachable = true;
                        get_cluster_info_sorted(last_place_idx, estimate, include_unreachable, ordering)
                            .into_iter()
                            .next()
                            .map(|visit_info| (estimate.0, estimate.1, visit_info))
                    })
                    .collect::<Vec<_>>();
                job_estimates.sort_by(|(_, _, a_info), (_, _, b_info)| ordering.deref()(a_info, b_info));

                // try to find the first successful addition to the cluster from job estimates
                let addition_result = unwrap_from_result(job_estimates.iter().try_fold(None, |_, candidate| {
                    try_add_job(
                        constraint,
                        last_place_idx,
                        &cluster,
                        (candidate.0, candidate.1),
                        config,
                        &center_commute,
                        check_insertion,
                    )
                    .map_or_else(
                        || {
                            cluster_candidates.remove(candidate.0);
                            Ok(None)
                        },
                        |data| Err(Some(data)),
                    )
                }));

                match addition_result {
                    Some((new_cluster, visit_info)) => {
                        if !matches!(config.visiting, VisitPolicy::Return) {
                            last_job = visit_info.job.clone();
                            last_place_idx = visit_info.place_idx;
                        }

                        count += 1;

                        cluster_candidates.remove(&visit_info.job);
                        cluster = with_cluster_dimension(new_cluster, visit_info);
                    }
                    None => cluster_candidates.clear(),
                }
            }

            if count > 1 {
                cluster = finish_cluster(cluster, config, &center_commute);
            }

            match (&best_cluster, count) {
                (_, count) if is_max_jobs(count) => Err(Some((cluster, count))),
                (Some((_, best_count)), _) if *best_count < count => Ok(Some((cluster, count))),
                (None, _) if count > 1 => Ok(Some((cluster, count))),
                _ => Ok(best_cluster),
            }
        },
    ))
    .map(|(cluster, _)| cluster)
}

fn try_add_job<F>(
    constraint: &ConstraintPipeline,
    center_place_idx: usize,
    cluster: &Job,
    candidate: (&Job, &Vec<DissimilarityInfo>),
    config: &ClusterConfig,
    center_commute: F,
    check_insertion: &CheckInsertionFn,
) -> Option<(Job, ClusterInfo)>
where
    F: Fn(&ClusterInfo) -> Commute,
{
    let time_window_threshold = config.threshold.smallest_time_window.unwrap_or(0.);
    let cluster = cluster.to_single();
    let cluster_place = cluster.places.first().expect("expect one place in cluster");
    let cluster_times = filter_times(cluster_place.times.as_slice());
    let cluster_last_duration = cluster
        .dimens
        .get_cluster()
        .and_then(|jobs| jobs.last())
        .and_then(|info| {
            info.job
                .as_single()
                .map(|job| (job, info))
                .and_then(|(job, info)| job.places.first().map(|place| (place, info)))
        })
        .map_or(cluster_place.duration, |(place, info)| {
            place.duration
                + if matches!(config.visiting, VisitPolicy::Return) { info.commute.backward.duration } else { 0. }
        });

    let job = candidate.0.to_single();
    let ordering = config.building.ordering_local.as_ref();
    let include_unreachable = true;
    let dissimilarities = get_cluster_info_sorted(center_place_idx, candidate, include_unreachable, ordering);

    unwrap_from_result(dissimilarities.into_iter().try_fold(None, |_, info| {
        let place = job.places.get(info.place_idx).expect("wrong place index");
        let place_times = filter_times(place.times.as_slice());

        // override backward movement costs in case of return
        let commute = if matches!(config.visiting, VisitPolicy::Return) {
            center_commute(&info)
        } else {
            Commute {
                forward: info.commute.forward,
                backward: CommuteInfo { location: place.location.expect("no location"), distance: 0., duration: 0. },
            }
        };
        let info = ClusterInfo { commute, ..info };

        let new_cluster_times = cluster_times
            .iter()
            .flat_map(|cluster_time| {
                place_times.iter().filter_map({
                    let info = info.clone();
                    move |place_time| {
                        // NOTE travel duration to the place can be deducted from its time window requirement
                        let place_time =
                            TimeWindow::new(place_time.start - info.commute.forward.duration, place_time.end);
                        let overlap_time = place_time.overlapping(cluster_time);

                        let duration = if place_time.end < cluster_time.end {
                            cluster_place.duration
                        } else {
                            cluster_last_duration
                        };

                        overlap_time.map(|time| (time, duration))
                    }
                })
            })
            .filter_map(|(overlap_time, duration)| {
                // TODO adapt service time from last cluster job to avoid time window violation of
                //      a next job in case of last time arrival. However, this can be too restrictive
                //      in some cases and can be improved to keep time window a bit wider.
                let end = overlap_time.end - duration - info.commute.forward.duration;
                if end - overlap_time.start < time_window_threshold {
                    None
                } else {
                    Some(TimeWindow::new(overlap_time.start, end))
                }
            })
            .collect::<Vec<_>>();

        // no time window intersection: cannot be clustered
        if new_cluster_times.is_empty() {
            return Ok(None);
        }

        let movement = match config.visiting {
            VisitPolicy::Return => info.commute.duration(),
            VisitPolicy::ClosedContinuation | VisitPolicy::OpenContinuation => info.commute.forward.duration,
        };

        let new_cluster_duration = cluster_place.duration + movement + info.service_time;

        let updated_cluster =
            create_single_job(cluster_place.location, new_cluster_duration, &new_cluster_times, &cluster.dimens);
        let updated_candidate =
            create_single_job(place.location, new_cluster_duration, &new_cluster_times, &job.dimens);

        constraint
            .merge_constrained(updated_cluster, updated_candidate)
            .and_then(|merged_cluster| check_insertion.deref()(&merged_cluster).map(|_| (merged_cluster, info)))
            .map(Some)
            .map_or_else(|_| Ok(None), Err)
    }))
}

fn get_cluster_info_sorted(
    center_place_idx: usize,
    estimate: (&Job, &Vec<DissimilarityInfo>),
    include_unreachable: bool,
    ordering: &(dyn Fn(&ClusterInfo, &ClusterInfo) -> Ordering + Send + Sync),
) -> Vec<ClusterInfo> {
    let (_, dissimilarities) = estimate;
    let mut dissimilarities = dissimilarities
        .iter()
        .filter(|(_, outer_place_idx, ..)| *outer_place_idx == center_place_idx)
        .filter(|(reachable, ..)| include_unreachable || *reachable)
        .map(|(_, _, info)| info.clone())
        .collect::<Vec<_>>();

    // sort dissimilarities based on user provided ordering function
    dissimilarities.sort_by(|a, b| ordering.deref()(a, b));

    dissimilarities
}

fn map_place(place_data: (PlaceIndex, &Place)) -> Option<PlaceInfo> {
    let (idx, place) = place_data;
    place.location.map(|location| (idx, location, place.duration, filter_times(place.times.as_slice())))
}

fn filter_times(times: &[TimeSpan]) -> Vec<TimeWindow> {
    times.iter().filter_map(|time| time.as_time_window()).collect::<Vec<_>>()
}

fn with_cluster_dimension(cluster: Job, visit_info: ClusterInfo) -> Job {
    let cluster = cluster.to_single();

    let mut cluster = Single { places: cluster.places.clone(), dimens: cluster.dimens.clone() };

    let mut jobs = cluster.dimens.get_cluster().cloned().unwrap_or_else(Vec::new);
    jobs.push(visit_info);

    cluster.dimens.set_cluster(jobs);

    Job::Single(Arc::new(cluster))
}

fn finish_cluster<F>(cluster: Job, config: &ClusterConfig, center_commute: F) -> Job
where
    F: Fn(&ClusterInfo) -> Commute,
{
    let clustered_jobs = cluster.dimens().get_cluster();

    match (&config.visiting, clustered_jobs) {
        (VisitPolicy::ClosedContinuation, Some(clustered)) => {
            // add extra duration from last clustered job to finish cluster visiting
            let cluster = cluster.to_single();
            assert_eq!(cluster.places.len(), 1);

            let mut clustered = clustered.clone();

            // NOTE add a return duration back to the cluster duration and modify backward info
            let last_info = clustered.last_mut().expect("empty cluster");
            let mut place = cluster.places.first().unwrap().clone();

            let commute = center_commute(last_info);
            place.duration += commute.backward.duration;
            last_info.commute.backward = commute.backward;

            let mut dimens = cluster.dimens.clone();
            dimens.set_cluster(clustered);

            Job::Single(Arc::new(Single { places: vec![place], dimens }))
        }
        _ => cluster,
    }
}

fn create_single_job(location: Option<Location>, duration: Duration, times: &[TimeWindow], dimens: &Dimensions) -> Job {
    Job::Single(Arc::new(Single {
        places: vec![Place {
            location,
            duration,
            times: times.iter().map(|time| TimeSpan::Window(time.clone())).collect(),
        }],
        dimens: dimens.clone(),
    }))
}

fn get_service_time(original: Duration, policy: &ServingPolicy) -> Duration {
    match policy {
        ServingPolicy::Original => original,
        ServingPolicy::Multiplier(multiplier) => original * *multiplier,
        ServingPolicy::Fixed(service_time) => *service_time,
    }
}
