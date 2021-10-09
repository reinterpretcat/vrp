use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::{Actor, Job, Place, Single, TransportCost};
use crate::models::Problem;
use crate::utils::{compare_floats, unwrap_from_result, Environment};
use hashbrown::{HashMap, HashSet};
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

const CLUSTER_DIMENSION_KEY: &str = "cls";

/// A trait to get or set cluster info.
pub trait ClusterDimension {
    /// Sets cluster.
    fn set_cluster(&mut self, jobs: Vec<Job>) -> &mut Self;
    /// Gets cluster.
    fn get_cluster(&self) -> Option<&Vec<Job>>;
}

impl ClusterDimension for Dimensions {
    fn set_cluster(&mut self, jobs: Vec<Job>) -> &mut Self {
        self.set_value(CLUSTER_DIMENSION_KEY, jobs);
        self
    }

    fn get_cluster(&self) -> Option<&Vec<Job>> {
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

/// Allows to control how clusters are built.
pub struct BuilderPolicy {
    /// Checks whether given cluster can get more.
    size_filter: Arc<dyn Fn(&[Job]) -> bool + Send + Sync>,
    /// Allows to select first clusters with desired properties.
    ordering: Arc<dyn Fn(&Job, &Job) -> Ordering + Send + Sync>,
}

/// Creates clusters of jobs trying to minimize their radius.
pub fn create_job_clusters(
    problem: Arc<Problem>,
    environment: Arc<Environment>,
    profile: &Profile,
    config: &ClusterConfig,
) -> Vec<(Job, Vec<Job>)> {
    let insertion_ctx = InsertionContext::new_empty(problem.clone(), environment);
    let check_job = get_check_job(&insertion_ctx, config.filtering.actor_filter.as_ref());
    let estimates = get_estimates(problem.as_ref(), profile, config);

    get_clusters(&insertion_ctx, estimates, config, &check_job)
}

type PlaceIndex = usize;
type DissimilarityInfo = (PlaceIndex, PlaceIndex, Distance, Duration);
type DissimilarityIndex = HashMap<Job, Vec<DissimilarityInfo>>;

/// Estimates ability of each job to build a cluster.
fn get_estimates(problem: &Problem, profile: &Profile, config: &ClusterConfig) -> HashMap<Job, DissimilarityIndex> {
    let transport = problem.transport.as_ref();
    let jobs = problem
        .jobs
        .all()
        .filter(&*config.filtering.job_filter)
        // NOTE multi-job is not supported
        .filter(|job| job.as_single().is_some())
        .collect::<Vec<_>>();

    jobs.iter()
        .map(|outer| {
            let dissimilarities = jobs
                .iter()
                .filter(|inner| outer != *inner)
                .filter_map(|inner| {
                    let dissimilarities = get_dissimilarities(&outer, inner, profile, &config.threshold, transport);
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
    profile: &Profile,
    threshold: &ThresholdPolicy,
    transport: &(dyn TransportCost + Send + Sync),
) -> Vec<DissimilarityInfo> {
    let departure = Default::default();
    outer
        .to_single()
        .places
        .iter()
        .enumerate()
        .filter_map(map_place)
        .flat_map(|(outer_place_idx, outer_loc, outer_times)| {
            inner.to_single().places.iter().enumerate().filter_map(map_place).filter_map(
                move |(inner_place_idx, inner_loc, inner_times)| {
                    let shared_time = outer_times
                        .iter()
                        .flat_map(|outer_time| {
                            inner_times.iter().filter_map(move |inner_time| {
                                outer_time.overlapping(inner_time).map(|tw| tw.duration())
                            })
                        })
                        .max_by(|a, b| compare_floats(*a, *b))
                        .unwrap_or(0.);

                    if shared_time > threshold.min_shared_time.unwrap_or(0.) {
                        let distance = transport.distance(profile, outer_loc, inner_loc, departure);
                        let duration = transport.duration(profile, outer_loc, inner_loc, departure);

                        match ((duration - threshold.moving_duration < 0.), (distance - threshold.moving_distance < 0.))
                        {
                            (true, true) => Some((outer_place_idx, inner_place_idx, shared_time, distance, duration)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                },
            )
        })
        //.max_by(|(_, _, left, _, _), (_, _, right, _, _)| compare_floats(*left, *right))
        .map(|(outer_place_idx, inner_place_idx, _, distance, duration)| {
            (outer_place_idx, inner_place_idx, distance, duration)
        })
        .collect()
}

fn get_check_job<'a>(
    insertion_ctx: &'a InsertionContext,
    actor_filter: &(dyn Fn(&Actor) -> bool + Send + Sync),
) -> impl Fn(&Job) -> bool + 'a {
    let result_selector = BestResultSelector::default();
    let routes = insertion_ctx
        .solution
        .registry
        .next()
        .filter(|route_ctx| actor_filter.deref()(&route_ctx.route.actor))
        .collect::<Vec<_>>();

    move |job: &Job| -> bool {
        unwrap_from_result(routes.iter().try_fold(false, |_, route_ctx| {
            let result = evaluate_job_insertion_in_route(
                &insertion_ctx,
                route_ctx,
                job,
                InsertionPosition::Any,
                InsertionResult::make_failure(),
                &result_selector,
            );

            match result {
                InsertionResult::Success(_) => Err(true),
                InsertionResult::Failure(_) => Ok(false),
            }
        }))
    }
}

fn get_clusters(
    insertion_ctx: &InsertionContext,
    estimates: HashMap<Job, DissimilarityIndex>,
    config: &ClusterConfig,
    check_job: &(dyn Fn(&Job) -> bool),
) -> Vec<(Job, Vec<Job>)> {
    let mut estimates = estimates
        .into_iter()
        .map(|(job, estimate)| (job, (None, estimate)))
        .collect::<Vec<(_, (Option<Job>, HashMap<_, _>))>>();
    let mut used_jobs = HashSet::new();
    let mut clusters = Vec::new();

    loop {
        // build clusters
        estimates.iter_mut().filter(|(_, (cluster, _))| cluster.is_none()).for_each(
            |(center, (cluster, candidates))| *cluster = build_job_cluster((center, candidates), config, check_job),
        );

        estimates.sort_by(|(_, (a, _)), (_, (b, _))| match (a, b) {
            (Some(a), Some(b)) => config.building.ordering.deref()(a, b),
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (None, None) => Ordering::Equal,
        });

        let new_cluster = estimates.first().and_then(|(_, (cluster, _))| cluster.as_ref()).cloned();

        if let Some(new_cluster) = new_cluster {
            let new_cluster_jobs = new_cluster.dimens().get_cluster().expect("expected to have jobs in a cluster");

            clusters.push((new_cluster.clone(), new_cluster_jobs.clone()));
            used_jobs.extend(new_cluster_jobs.iter().cloned());

            let new_cluster_jobs = new_cluster_jobs.iter().collect::<HashSet<_>>();

            // remove used jobs from analysis
            estimates.retain(|(center, _)| !new_cluster_jobs.contains(center));
            estimates.iter_mut().for_each(|(_, (cluster, candidates))| {
                candidates.retain(|job, _| !new_cluster_jobs.contains(job));

                let is_cluster_affected = cluster
                    .as_ref()
                    .and_then(|cluster| cluster.dimens().get_cluster())
                    .map_or(false, |cluster_jobs| cluster_jobs.iter().any(|job| new_cluster_jobs.contains(job)));

                if is_cluster_affected {
                    // NOTE force to rebuild cluster on next iteration
                    *cluster = None;
                }
            });
            estimates.retain(|(_, (_, candidates))| !candidates.is_empty());
        } else {
            break;
        }
    }

    clusters
}

fn build_job_cluster(
    estimate: (&Job, &DissimilarityIndex),
    config: &ClusterConfig,
    check_job: &(dyn Fn(&Job) -> bool),
) -> Option<Job> {
    let (center, candidates) = estimate;

    unimplemented!()
}

fn map_place(place_info: (PlaceIndex, &Place)) -> Option<(PlaceIndex, Location, Vec<TimeWindow>)> {
    let (idx, place) = place_info;
    place.location.map(|location| {
        (idx, location, place.times.iter().filter_map(|time| time.as_time_window()).collect::<Vec<_>>())
    })
}

fn deep_copy(job: &Job) -> Job {
    match job {
        Job::Single(single) => {
            Job::Single(Arc::new(Single { places: single.places.clone(), dimens: single.dimens.clone() }))
        }
        Job::Multi(_) => unimplemented!(),
    }
}
