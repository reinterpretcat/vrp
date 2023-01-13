//! This module provides functionality to automatically check that given solution is feasible
//! which means that there is no constraint violations.

#[cfg(test)]
#[path = "../../tests/unit/checker/checker_test.rs"]
mod checker_test;

use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::{CoordIndex, Location};
use crate::parse_time;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;
use vrp_core::construction::clustering::vicinity::ClusterConfig;
use vrp_core::construction::clustering::vicinity::VisitPolicy;
use vrp_core::models::common::{Duration, Profile, TimeWindow};
use vrp_core::models::solution::{Commute as DomainCommute, CommuteInfo as DomainCommuteInfo};
use vrp_core::models::Problem as CoreProblem;
use vrp_core::solver::processing::VicinityDimension;

/// Stores problem and solution together and provides some helper methods.
pub struct CheckerContext {
    /// An original problem definition.
    pub problem: Problem,
    /// Routing matrices.
    pub matrices: Option<Vec<Matrix>>,
    /// Solution to be checked
    pub solution: Solution,

    job_map: HashMap<String, Job>,
    coord_index: CoordIndex,
    profile_index: HashMap<String, usize>,
    core_problem: Arc<CoreProblem>,
    clustering: Option<ClusterConfig>,
}

/// Represents all possible activity types.
enum ActivityType {
    Terminal,
    Job(Job),
    Depot(VehicleDispatch),
    Break(VehicleBreak),
    Reload(VehicleReload),
}

impl CheckerContext {
    /// Creates an instance of `CheckerContext`
    pub fn new(
        core_problem: Arc<CoreProblem>,
        problem: Problem,
        matrices: Option<Vec<Matrix>>,
        solution: Solution,
    ) -> Result<Self, Vec<String>> {
        let job_map = problem.plan.jobs.iter().map(|job| (job.id.clone(), job.clone())).collect();
        let clustering = core_problem.extras.get_cluster_config().cloned();
        let coord_index = CoordIndex::new(&problem);
        let profile_index = if matrices.is_none() {
            HashMap::new()
        } else {
            get_matrices(&matrices)
                .and_then(|matrices| get_profile_index(&problem, matrices.as_slice()))
                .map_err(|err| vec![err])?
        };

        Ok(Self { problem, matrices, solution, job_map, coord_index, profile_index, core_problem, clustering })
    }

    /// Performs solution check.
    pub fn check(&self) -> Result<(), Vec<String>> {
        // avoid duplicates keeping original order
        let (_, errors) = check_vehicle_load(self)
            .err()
            .into_iter()
            .chain(check_relations(self).err().into_iter())
            .chain(check_breaks(self).err().into_iter())
            .chain(check_assignment(self).err().into_iter())
            .chain(check_routing(self).err().into_iter())
            .chain(check_limits(self).err().into_iter())
            .flatten()
            .fold((HashSet::new(), Vec::default()), |(mut used, mut errors), error| {
                if !used.contains(&error) {
                    errors.push(error.clone());
                    used.insert(error);
                }

                (used, errors)
            });

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Gets vehicle by its id.
    fn get_vehicle(&self, vehicle_id: &str) -> Result<&VehicleType, String> {
        self.problem
            .fleet
            .vehicles
            .iter()
            .find(|v| v.vehicle_ids.contains(&vehicle_id.to_string()))
            .ok_or_else(|| format!("cannot find vehicle with id '{}'", vehicle_id))
    }

    fn get_vehicle_profile(&self, vehicle_id: &str) -> Result<Profile, String> {
        let profile = &self.get_vehicle(vehicle_id)?.profile;
        let index = self
            .profile_index
            .get(profile.matrix.as_str())
            .cloned()
            .ok_or(format!("cannot get matrix for '{}' profile", profile.matrix))?;

        Ok(Profile { index, scale: profile.scale.unwrap_or(1.) })
    }

    /// Gets activity operation time range in seconds since Unix epoch.
    fn get_activity_time(&self, stop: &Stop, activity: &Activity) -> TimeWindow {
        let schedule = stop.schedule();

        let time = activity
            .time
            .clone()
            .unwrap_or_else(|| Interval { start: schedule.arrival.clone(), end: schedule.departure.clone() });

        TimeWindow::new(parse_time(&time.start), parse_time(&time.end))
    }

    /// Gets activity location.
    fn get_activity_location(&self, stop: &Stop, activity: &Activity) -> Option<Location> {
        activity.location.clone().or_else(|| match stop {
            Stop::Point(stop) => Some(stop.location.clone()),
            Stop::Transit(_) => None,
        })
    }

    /// Gets vehicle shift where activity is used.
    fn get_vehicle_shift(&self, tour: &Tour) -> Result<VehicleShift, String> {
        let tour_time = TimeWindow::new(
            parse_time(
                &tour.stops.first().as_ref().ok_or_else(|| "cannot get first activity".to_string())?.schedule().arrival,
            ),
            parse_time(
                &tour.stops.last().as_ref().ok_or_else(|| "cannot get last activity".to_string())?.schedule().arrival,
            ),
        );

        self.get_vehicle(&tour.vehicle_id)?
            .shifts
            .iter()
            .find(|shift| {
                let shift_time = TimeWindow::new(
                    parse_time(&shift.start.earliest),
                    shift.end.as_ref().map_or_else(|| f64::MAX, |place| parse_time(&place.latest)),
                );
                shift_time.intersects(&tour_time)
            })
            .cloned()
            .ok_or_else(|| format!("cannot find shift for tour with vehicle if: '{}'", tour.vehicle_id))
    }

    /// Returns stop's activity type names.
    fn get_stop_activity_types(&self, stop: &Stop) -> Vec<String> {
        stop.activities().iter().map(|a| a.activity_type.clone()).collect()
    }

    /// Gets wrapped activity type.
    fn get_activity_type(&self, tour: &Tour, stop: &Stop, activity: &Activity) -> Result<ActivityType, String> {
        let shift = self.get_vehicle_shift(tour)?;
        let time = self.get_activity_time(stop, activity);
        let location = self.get_activity_location(stop, activity);

        match activity.activity_type.as_str() {
            "departure" | "arrival" => Ok(ActivityType::Terminal),
            "pickup" | "delivery" | "service" | "replacement" => {
                self.job_map.get(activity.job_id.as_str()).map_or_else(
                    || Err(format!("cannot find job with id '{}'", activity.job_id)),
                    |job| Ok(ActivityType::Job(job.clone())),
                )
            }
            "break" => shift
                .breaks
                .as_ref()
                .and_then(|breaks| {
                    breaks.iter().find(|b| {
                        match b {
                            VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeWindow(tw), .. } => {
                                parse_time_window(tw).intersects(&time)
                            }
                            VehicleBreak::Optional { time: VehicleOptionalBreakTime::TimeOffset(offset), .. } => {
                                assert_eq!(offset.len(), 2);
                                // NOTE make expected time window wider due to reschedule departure
                                let stops = &tour.stops;
                                let schedule = &stops.first().unwrap().schedule();
                                let start = parse_time(&schedule.arrival) + *offset.first().unwrap();
                                let end = parse_time(&schedule.departure) + *offset.last().unwrap();

                                TimeWindow::new(start, end).intersects(&time)
                            }
                            VehicleBreak::Required { time: VehicleRequiredBreakTime::ExactTime(b_time), duration } => {
                                let start = parse_time(b_time);
                                let end = start + *duration;

                                TimeWindow::new(start, end).intersects(&time)
                            }
                            VehicleBreak::Required { time: VehicleRequiredBreakTime::OffsetTime(offset), duration } => {
                                let departure = parse_time(&tour.stops.first().unwrap().schedule().departure);
                                let start = departure + *offset;
                                let end = start + *duration;

                                TimeWindow::new(start, end).intersects(&time)
                            }
                        }
                    })
                })
                .map(|b| ActivityType::Break(b.clone()))
                .ok_or_else(|| format!("cannot find break for tour '{}'", tour.vehicle_id)),
            "reload" => shift
                .reloads
                .as_ref()
                // TODO match reload's time windows
                .and_then(|reload| {
                    reload.iter().find(|r| {
                        location.as_ref().map_or(false, |location| r.location == *location) && r.tag == activity.job_tag
                    })
                })
                .map(|r| ActivityType::Reload(r.clone()))
                .ok_or_else(|| format!("cannot find reload for tour '{}'", tour.vehicle_id)),
            "dispatch" => shift
                .dispatch
                .as_ref()
                .and_then(|dispatch| {
                    dispatch.iter().find(|d| location.as_ref().map_or(false, |location| d.location == *location))
                })
                .map(|d| ActivityType::Depot(d.clone()))
                .ok_or_else(|| format!("cannot find dispatch for tour '{}'", tour.vehicle_id)),
            _ => Err(format!("unknown activity type: '{}'", activity.activity_type)),
        }
    }

    fn get_job_by_id(&self, job_id: &str) -> Option<&Job> {
        self.problem.plan.jobs.iter().find(|job| job.id == job_id)
    }

    fn get_commute_info(
        &self,
        profile: Option<Profile>,
        parking: Duration,
        stop: &PointStop,
        activity_idx: usize,
    ) -> Result<Option<DomainCommute>, String> {
        let get_activity_location_by_idx = |idx: usize| {
            stop.activities
                .get(idx)
                .and_then(|activity| activity.location.as_ref())
                .and_then(|location| self.get_location_index(location).ok())
        };

        let get_activity_commute_by_idx = |idx: usize| -> Option<DomainCommute> {
            stop.activities
                .get(idx)
                .and_then(|activity| activity.commute.as_ref())
                .map(|commute| commute.to_domain(&self.coord_index))
        };

        match (&self.clustering, &profile, get_activity_commute_by_idx(activity_idx)) {
            (Some(config), Some(profile), Some(commute)) => {
                let stop_location = self.get_location_index(&stop.location).ok();
                // NOTE we don't check whether zero time commute is correct here
                match (commute.is_zero_distance(), activity_idx) {
                    (true, _) => Ok(Some(commute)),
                    // NOTE that's unreachable
                    (false, idx) if idx == 0 => Err("cannot have commute at first activity in the stop".to_string()),
                    (false, idx) => {
                        let prev_location = if matches!(config.visiting, VisitPolicy::Return) {
                            stop_location
                        } else {
                            get_activity_location_by_idx(idx - 1)
                        };
                        let curr_location = get_activity_location_by_idx(idx);

                        match (curr_location, prev_location) {
                            (Some(curr_location), Some(prev_location)) => {
                                let (f_distance, f_duration) =
                                    self.get_matrix_data(profile, prev_location, curr_location)?;

                                let has_next_commute = get_activity_location_by_idx(idx + 1)
                                    .zip(get_activity_commute_by_idx(idx + 1))
                                    .is_some();
                                let (b_location, b_distance, b_duration) = match (&config.visiting, has_next_commute) {
                                    (VisitPolicy::Return, _) | (VisitPolicy::ClosedContinuation, false) => {
                                        let stop_location = stop_location.ok_or("no location for clustered stop")?;
                                        let (b_distance, b_duration) =
                                            self.get_matrix_data(profile, curr_location, stop_location)?;

                                        (stop_location, b_distance, b_duration)
                                    }
                                    (VisitPolicy::OpenContinuation, _) | (VisitPolicy::ClosedContinuation, true) => {
                                        (curr_location, 0_i64, 0_i64)
                                    }
                                };

                                // NOTE parking correction
                                let f_duration = if f_duration == 0 { parking } else { f_duration as f64 };

                                Ok(Some(DomainCommute {
                                    forward: DomainCommuteInfo {
                                        location: prev_location,
                                        distance: f_distance as f64,
                                        duration: f_duration,
                                    },
                                    backward: DomainCommuteInfo {
                                        location: b_location,
                                        distance: b_distance as f64,
                                        duration: b_duration as f64,
                                    },
                                }))
                            }
                            _ => Err("cannot find next commute info".to_string()),
                        }
                    }
                }
            }
            _ => Ok(None),
        }
    }

    fn visit_job<F1, F2, R>(
        &self,
        activity: &Activity,
        activity_type: &ActivityType,
        job_visitor: F1,
        other_visitor: F2,
    ) -> Result<R, String>
    where
        F1: Fn(&Job, &JobTask) -> R,
        F2: Fn() -> R,
    {
        match activity_type {
            ActivityType::Job(job) => {
                let pickups = job_task_size(&job.pickups);
                let deliveries = job_task_size(&job.deliveries);
                let tasks = pickups + deliveries + job_task_size(&job.services) + job_task_size(&job.replacements);

                if tasks < 2 || (tasks == 2 && pickups == 1 && deliveries == 1) {
                    match_job_task(activity.activity_type.as_str(), job, |tasks| tasks.first())
                } else {
                    activity.job_tag.as_ref().ok_or_else(|| {
                        format!("checker requires that multi job activity must have tag: '{}'", activity.job_id)
                    })?;

                    match_job_task(activity.activity_type.as_str(), job, |tasks| {
                        tasks.iter().find(|task| task.places.iter().any(|place| place.tag == activity.job_tag))
                    })
                }
                .map(|task| job_visitor(job, task))
            }
            .ok_or_else(|| "cannot match activity to job place".to_string()),
            _ => Ok(other_visitor()),
        }
    }

    fn get_location_index(&self, location: &Location) -> Result<usize, String> {
        self.coord_index
            .get_by_loc(location)
            .ok_or_else(|| format!("cannot find coordinate in coord index: {:?}", location))
    }

    fn get_matrix_data(&self, profile: &Profile, from_idx: usize, to_idx: usize) -> Result<(i64, i64), String> {
        let matrices = get_matrices(&self.matrices)?;
        let matrix =
            matrices.get(profile.index).ok_or_else(|| format!("cannot find matrix with index {}", profile.index))?;

        let matrix_size = get_matrix_size(matrices.as_slice());
        let matrix_idx = from_idx * matrix_size + to_idx;

        let distance = get_matrix_value(matrix_idx, &matrix.distances)?;
        let duration = get_matrix_value(matrix_idx, &matrix.travel_times)?;
        let duration = (duration as f64 * profile.scale) as i64;

        Ok((distance, duration))
    }
}

fn job_task_size(tasks: &Option<Vec<JobTask>>) -> usize {
    tasks.as_ref().map_or(0, |p| p.len())
}

fn match_job_task<'a>(
    activity_type: &str,
    job: &'a Job,
    tasks_fn: impl Fn(&'a Vec<JobTask>) -> Option<&'a JobTask>,
) -> Option<&'a JobTask> {
    let tasks = match activity_type {
        "pickup" => job.pickups.as_ref(),
        "delivery" => job.deliveries.as_ref(),
        "service" => job.services.as_ref(),
        "replacement" => job.replacements.as_ref(),
        _ => None,
    };

    tasks.and_then(tasks_fn)
}

fn parse_time_window(tw: &[String]) -> TimeWindow {
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_time_window(stop: &Stop, activity: &Activity) -> TimeWindow {
    let schedule = stop.schedule();
    let (start, end) = activity
        .time
        .as_ref()
        .map_or_else(|| (&schedule.arrival, &schedule.departure), |interval| (&interval.start, &interval.end));

    TimeWindow::new(parse_time(start), parse_time(end))
}

fn get_matrix_size(matrices: &[Matrix]) -> usize {
    (matrices.first().unwrap().travel_times.len() as f64).sqrt().round() as usize
}

fn get_matrix_value(idx: usize, matrix_values: &[i64]) -> Result<i64, String> {
    matrix_values
        .get(idx)
        .cloned()
        .ok_or_else(|| format!("attempt to get value out of bounds: {} vs {}", idx, matrix_values.len()))
}

fn get_matrices(matrices: &Option<Vec<Matrix>>) -> Result<&Vec<Matrix>, String> {
    let matrices = matrices.as_ref().unwrap();

    if matrices.iter().any(|matrix| matrix.timestamp.is_some()) {
        return Err("not implemented: time aware routing check".to_string());
    }

    Ok(matrices)
}

fn get_profile_index(problem: &Problem, matrices: &[Matrix]) -> Result<HashMap<String, usize>, String> {
    let profiles = problem.fleet.profiles.len();
    if profiles != matrices.len() {
        return Err(format!(
            "precondition failed: amount of matrices supplied ({}) does not match profile specified ({})",
            matrices.len(),
            profiles,
        ));
    }

    Ok(problem
        .fleet
        .profiles
        .iter()
        .enumerate()
        .map(|(idx, profile)| (profile.name.to_string(), idx))
        .collect::<HashMap<_, _>>())
}

mod assignment;
use crate::checker::assignment::check_assignment;

mod capacity;
use crate::checker::capacity::check_vehicle_load;

mod limits;
use crate::checker::limits::check_limits;

mod breaks;
use crate::checker::breaks::check_breaks;

mod relations;
use crate::checker::relations::check_relations;

mod routing;
use crate::checker::routing::check_routing;
