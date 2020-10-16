use crate::format::{CoordIndex, JobIndex};
use crate::parse_time;
use std::iter::once;
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::{Activity, Place};

use crate::format::solution::Activity as FormatActivity;
use crate::format::solution::Stop as FormatStop;
use crate::format::solution::Tour as FormatTour;
use hashbrown::HashSet;

/// Aggregates job specific information for a job activity.
pub(crate) struct JobInfo(pub Job, pub Arc<Single>, pub Place, pub TimeWindow);

/// Tries to match given activity to core job models. None is returned in case of
/// non-job activity (departure, arrival).
pub(crate) fn try_match_job(
    tour: &FormatTour,
    stop: &FormatStop,
    activity: &FormatActivity,
    job_index: &JobIndex,
    coord_index: &CoordIndex,
) -> Result<Option<JobInfo>, String> {
    let ctx = ActivityContext {
        route_start_time: tour
            .stops
            .first()
            .map(|stop| parse_time(&stop.time.departure))
            .ok_or_else(|| "empty route".to_owned())?,
        location: coord_index
            .get_by_loc(activity.location.as_ref().unwrap_or(&stop.location))
            .ok_or_else(|| format!("cannot get location for activity for job '{}'", activity.job_id))?,
        time: activity
            .time
            .as_ref()
            .map(|time| TimeWindow::new(parse_time(&time.start), parse_time(&time.end)))
            .unwrap_or_else(|| TimeWindow::new(parse_time(&stop.time.arrival), parse_time(&stop.time.departure))),
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
                    let tags = multi.jobs.iter().filter_map(|job| get_tag(job).cloned()).collect::<HashSet<_>>();
                    if tags.len() < multi.jobs.len() {
                        return Err(format!(
                            "cannot check multi job without unique tags, check '{}' job",
                            activity.job_id
                        ));
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
        "break" | "depot" | "reload" => Ok(Some(
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
        _ => Err(format!("unknown activity type: {}", activity.activity_type)),
    }
}

struct ActivityContext<'a> {
    route_start_time: Timestamp,
    location: Location,
    time: TimeWindow,
    act_type: &'a String,
    job_id: &'a String,
    tag: Option<&'a String>,
}

fn match_place<'a>(single: &Arc<Single>, is_job_activity: bool, activity_ctx: &'a ActivityContext) -> Option<Place> {
    let job_id = get_job_id(single);
    let is_same_ids = *activity_ctx.job_id == job_id;
    let is_same_tags = match (get_tag(single), activity_ctx.tag) {
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
                let time = place
                    .times
                    .iter()
                    .find(|time| time.intersects(activity_ctx.route_start_time, &activity_ctx.time))
                    .unwrap();

                let time = match time {
                    TimeSpan::Window(tw) => tw.clone(),
                    // NOTE we don't know when original start should be
                    TimeSpan::Offset(offset) => {
                        TimeWindow::new(activity_ctx.time.start, activity_ctx.route_start_time + offset.end)
                    }
                };

                Place { location: activity_ctx.location, duration: place.duration, time }
            }),
        _ => None,
    }
}

fn get_tag(single: &Single) -> Option<&String> {
    single.dimens.get_value::<String>("tag")
}

fn get_job_id(single: &Arc<Single>) -> String {
    Activity {
        place: Place { location: 0, duration: 0.0, time: TimeWindow::new(0., 0.) },
        schedule: Schedule { arrival: 0.0, departure: 0.0 },
        job: Some(single.clone()),
    }
    .retrieve_job()
    .unwrap()
    .dimens()
    .get_id()
    .cloned()
    .expect("cannot get job id")
}
