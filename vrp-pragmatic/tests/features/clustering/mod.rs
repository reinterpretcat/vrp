use crate::format::solution::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::common::{Distance, Timestamp};

type CommuteData = Option<(Distance, Timestamp, Timestamp)>;

struct ActivityData {
    job_id: String,
    location: Option<f64>,
    a_type: String,
    time: Option<(Timestamp, Timestamp)>,
    commute: Option<(CommuteData, CommuteData)>,
}

impl ActivityData {
    pub fn new(
        data: (&str, Option<f64>, &str, Option<(Timestamp, Timestamp)>, Option<(CommuteData, CommuteData)>),
    ) -> Self {
        Self { job_id: data.0.to_string(), location: data.1, a_type: data.2.to_string(), time: data.3, commute: data.4 }
    }
}

impl From<ActivityData> for Activity {
    fn from(activity: ActivityData) -> Self {
        let convert_expected_commute_info = |commute: Option<(f64, f64, f64)>| {
            commute.map(|commute| CommuteInfo {
                distance: commute.0,
                time: Interval { start: format_time(commute.1), end: format_time(commute.2) },
            })
        };

        Activity {
            job_id: activity.job_id,
            activity_type: activity.a_type,
            location: activity.location.map(|loc| vec![loc, 0.].to_loc()),
            time: activity.time.map(|(start, end)| Interval { start: format_time(start), end: format_time(end) }),
            job_tag: None,
            commute: activity.commute.map(|(fwd, bak)| Commute {
                forward: convert_expected_commute_info(fwd),
                backward: convert_expected_commute_info(bak),
            }),
        }
    }
}

struct StopData {
    location: Location,
    distance: i64,
    load: i32,
    time: (Timestamp, Timestamp),
    activities: Vec<ActivityData>,
}

impl StopData {
    pub fn new(data: (f64, i64, i32, (Timestamp, Timestamp), Vec<ActivityData>)) -> Self {
        Self { location: vec![data.0, 0.].to_loc(), distance: data.1, load: data.2, time: data.3, activities: data.4 }
    }
}

impl From<StopData> for Stop {
    fn from(stop: StopData) -> Self {
        Stop {
            location: stop.location,
            time: Schedule { arrival: format_time(stop.time.0), departure: format_time(stop.time.1) },
            distance: stop.distance,
            load: vec![stop.load],
            activities: stop.activities.into_iter().map(ActivityData::into).collect(),
        }
    }
}

mod basic_vicinity_test;
