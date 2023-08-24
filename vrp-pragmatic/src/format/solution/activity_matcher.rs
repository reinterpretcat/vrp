use crate::construction::enablers::JobTie;
use crate::format::problem::VehicleBreak;
use crate::format::problem::{Problem as FormatProblem, VehicleRequiredBreakTime};
use crate::format::solution::{Activity as FormatActivity, Schedule as FormatSchedule, Tour as FormatTour};
use crate::format::solution::{PointStop, TransitStop};
use crate::format::{CoordIndex, JobIndex};
use crate::parse_time;
use hashbrown::HashSet;
use std::cmp::Ordering;
use std::iter::once;
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::{Activity, Place};
use vrp_core::prelude::*;

/// Aggregates job specific information for a job activity.
pub(crate) struct JobInfo(pub Job, pub Arc<Single>, pub Place, pub TimeWindow);

/// Tries to match given activity to core job models. None is returned in case of
/// non-job activity (departure, arrival).
pub(crate) fn try_match_point_job(
    tour: &FormatTour,
    stop: &PointStop,
    activity: &FormatActivity,
    job_index: &JobIndex,
    coord_index: &CoordIndex,
) -> Result<Option<JobInfo>, GenericError> {
    let ctx = ActivityContext {
        route_start_time: get_route_start_time(tour)?,
        location: coord_index
            .get_by_loc(activity.location.as_ref().unwrap_or(&stop.location))
            .ok_or_else(|| format!("cannot get location for activity for job '{}'", activity.job_id))?,
        time: get_activity_time(activity, &stop.time),
        act_type: &activity.activity_type,
        job_id: &activity.job_id,
        tag: activity.job_tag.as_ref(),
    };

    match activity.activity_type.as_str() {
        "departure" | "arrival" => Ok(None),
        "pickup" | "delivery" | "replacement" | "service" => {
            let job =
                job_index.get(&activity.job_id).ok_or_else(|| format!("unknown job id: '{}'", activity.job_id))?;
            let singles: Box<dyn Iterator<Item = &Arc<_>>> = match job {
                Job::Single(single) => Box::new(once(single)),
                Job::Multi(multi) => {
                    let tags = multi
                        .jobs
                        .iter()
                        .filter_map(|single| single.dimens.get_place_tags())
                        .flat_map(|tags| tags.iter().map(|(_, tag)| tag))
                        .collect::<HashSet<_>>();
                    if tags.len() < multi.jobs.len() {
                        return Err(format!(
                            "cannot check multi job without unique tags, check '{}' job",
                            activity.job_id
                        )
                        .into());
                    }

                    Box::new(multi.jobs.iter())
                }
            };
            let (single, place) = singles
                .filter_map(|single| match_place(single, true, &ctx).map(|place| (single, place)))
                .next()
                .ok_or_else(|| format!("cannot match job '{}'", activity.job_id))?;

            Ok(Some(JobInfo(job.clone(), single.clone(), place, ctx.time)))
        }
        "break" | "dispatch" | "reload" | "recharge" => Ok(Some(
            (1..)
                .map(|idx| format!("{}_{}_{}_{}", tour.vehicle_id, activity.activity_type, tour.shift_index, idx))
                .map(|job_id| job_index.get(&job_id))
                .take_while(|job| job.is_some())
                .filter_map(|job| job.and_then(|job| job.as_single().map(|s| (job.clone(), s.clone()))))
                .filter_map(|(job, single)| {
                    match_place(&single, false, &ctx).map(|place| JobInfo(job, single, place, ctx.time.clone()))
                })
                .next()
                .ok_or_else(|| format!("cannot match '{}' for '{}'", ctx.act_type, tour.vehicle_id))?,
        )),
        _ => Err(format!("unknown activity type: {}", activity.activity_type).into()),
    }
}

/// Tries to return activity from transit stop to a break.
pub(crate) fn try_match_transit_activity(
    problem: &FormatProblem,
    tour: &FormatTour,
    stop: &TransitStop,
    activity: &FormatActivity,
) -> Result<TimeWindow, GenericError> {
    try_match_break_activity(problem, tour, &stop.time, activity)
}

/// Tries to match break activity.
pub(crate) fn try_match_break_activity(
    problem: &FormatProblem,
    tour: &FormatTour,
    stop_schedule: &FormatSchedule,
    activity: &FormatActivity,
) -> Result<TimeWindow, GenericError> {
    let route_start_time = get_route_start_time(tour)?;
    let activity_time = get_activity_time(activity, stop_schedule);

    problem
        .fleet
        .vehicles
        .iter()
        .flat_map(|vehicle| vehicle.shifts.iter())
        .flat_map(|shift| shift.breaks.iter())
        .flat_map(|brs| brs.iter())
        .filter_map(|br| match br {
            VehicleBreak::Required { time: VehicleRequiredBreakTime::ExactTime { earliest, latest }, duration } => {
                Some(TimeWindow::new(parse_time(earliest), parse_time(latest) + *duration))
            }
            VehicleBreak::Required { time: VehicleRequiredBreakTime::OffsetTime { earliest, latest }, duration } => {
                Some(TimeWindow::new(route_start_time + *earliest, route_start_time + *latest + *duration))
            }
            VehicleBreak::Optional { .. } => None,
        })
        .find(|time| activity_time.intersects(time))
        .ok_or_else(|| "cannot match activity to required break".into())
}

struct ActivityContext<'a> {
    route_start_time: Timestamp,
    location: Location,
    time: TimeWindow,
    act_type: &'a String,
    job_id: &'a String,
    tag: Option<&'a String>,
}

fn match_place(single: &Arc<Single>, is_job_activity: bool, activity_ctx: &ActivityContext) -> Option<Place> {
    let job_id = get_job_id(single);
    let job_tag =
        get_job_tag(single, (activity_ctx.location, (activity_ctx.time.clone(), activity_ctx.route_start_time)));

    let is_same_ids = *activity_ctx.job_id == job_id;
    let is_same_tags = match (job_tag, activity_ctx.tag) {
        (Some(job_tag), Some(activity_tag)) => job_tag == activity_tag,
        (None, None) => true,
        _ => false,
    };

    match (is_same_tags, is_same_ids, is_job_activity) {
        (true, false, true) => None,
        (true, true, _) | (true, false, false) => single
            .places
            .iter()
            .find(|place| {
                let is_same_location = place.location.map_or(true, |l| l == activity_ctx.location);
                let is_proper_time =
                    place.times.iter().any(|time| time.intersects(activity_ctx.route_start_time, &activity_ctx.time));

                is_same_location && is_proper_time
            })
            .map(|place| {
                // NOTE search for the latest occurrence assuming that times are sorted
                let time = place
                    .times
                    .iter()
                    .rfind(|time| time.intersects(activity_ctx.route_start_time, &activity_ctx.time))
                    .unwrap();

                let time = match time {
                    TimeSpan::Window(tw) => tw.clone(),
                    TimeSpan::Offset(_) => {
                        TimeWindow::new(activity_ctx.time.end - place.duration, activity_ctx.time.end)
                    }
                };

                Place { location: activity_ctx.location, duration: place.duration, time }
            }),
        _ => None,
    }
}

pub(crate) fn get_job_tag(single: &Single, place: (Location, (TimeWindow, Timestamp))) -> Option<&String> {
    let (location, (time_window, start_time)) = place;
    single.dimens.get_place_tags().map(|tags| (tags, &single.places)).and_then(|(tags, places)| {
        tags.iter()
            .find(|(place_idx, _)| {
                let place = places.get(*place_idx).expect("invalid tag place index");

                let is_correct_location = place.location.map_or(true, |l| location == l);
                let is_correct_time = place
                    .times
                    .iter()
                    .map(|time| time.to_time_window(start_time))
                    .any(|time| time.intersects(&time_window));

                // TODO check duration too?

                is_correct_location && is_correct_time
            })
            .map(|(_, tag)| tag)
    })
}

pub(crate) fn get_extra_time(stop: &PointStop, activity: &FormatActivity, place: &Place) -> Option<f64> {
    let activity_time = get_activity_time(activity, &stop.time);
    stop.activities
        .iter()
        .filter_map(|a| {
            if a.activity_type == "break" && a != activity {
                a.time.as_ref().and_then(|time| {
                    let break_time = TimeWindow::new(parse_time(&time.start), parse_time(&time.end));
                    let activity_time = TimeWindow::new(activity_time.start, activity_time.start + place.duration);
                    activity_time
                        .overlapping(&break_time)
                        .filter(|overlap| compare_floats(overlap.start, overlap.end) != Ordering::Equal)
                        .map(|overlap| break_time.end - overlap.end + overlap.duration())
                })
            } else {
                None
            }
        })
        .next()
}

fn get_job_id(single: &Arc<Single>) -> String {
    Activity {
        place: Place { location: 0, duration: 0.0, time: TimeWindow::new(0., 0.) },
        schedule: Schedule { arrival: 0.0, departure: 0.0 },
        job: Some(single.clone()),
        commute: None,
    }
    .retrieve_job()
    .unwrap()
    .dimens()
    .get_job_id()
    .cloned()
    .expect("cannot get job id")
}

fn get_activity_time(activity: &FormatActivity, stop_schedule: &FormatSchedule) -> TimeWindow {
    activity
        .time
        .as_ref()
        .map(|time| TimeWindow::new(parse_time(&time.start), parse_time(&time.end)))
        .unwrap_or_else(|| TimeWindow::new(parse_time(&stop_schedule.arrival), parse_time(&stop_schedule.departure)))
}

fn get_route_start_time(tour: &FormatTour) -> Result<Timestamp, GenericError> {
    tour.stops.first().map(|stop| parse_time(&stop.schedule().departure)).ok_or_else(|| "empty route".into())
}
