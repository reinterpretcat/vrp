use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::common::{Distance, Timestamp};

type CommuteData = Option<(f64, Distance, Timestamp, Timestamp)>;

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
        let convert_expected_commute_info = |commute: CommuteData| {
            commute.map(|commute| CommuteInfo {
                location: vec![commute.0, 0.].to_loc(),
                distance: commute.1,
                time: Interval { start: format_time(commute.2), end: format_time(commute.3) },
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
    parking: i64,
    time: (Timestamp, Timestamp),
    activities: Vec<ActivityData>,
}

impl StopData {
    pub fn new(data: (f64, i64, i32, i64, (Timestamp, Timestamp), Vec<ActivityData>)) -> Self {
        Self {
            location: vec![data.0, 0.].to_loc(),
            distance: data.1,
            load: data.2,
            parking: data.3,
            time: data.4,
            activities: data.5,
        }
    }
}

impl From<StopData> for Stop {
    fn from(stop: StopData) -> Self {
        Stop {
            location: stop.location,
            time: Schedule { arrival: format_time(stop.time.0), departure: format_time(stop.time.1) },
            distance: stop.distance,
            parking: if stop.parking > 0 {
                Some(Interval { start: format_time(stop.time.0), end: format_time(stop.time.0 + stop.parking as f64) })
            } else {
                None
            },
            load: vec![stop.load],
            activities: stop.activities.into_iter().map(ActivityData::into).collect(),
        }
    }
}

fn create_statistic(data: (f64, i64, i64, (i64, i64, i64, i64))) -> Statistic {
    Statistic {
        cost: data.0,
        distance: data.1,
        duration: data.2,
        times: Timing {
            driving: data.3 .0,
            serving: data.3 .1,
            commuting: data.3 .2,
            parking: data.3 .3,
            ..Timing::default()
        },
    }
}

fn create_test_problem(jobs_data: &[(f64, &str)], capacity: i32, clustering: Clustering) -> Problem {
    Problem {
        plan: Plan {
            jobs: jobs_data
                .iter()
                .enumerate()
                .map(|(idx, &(loc, j_type))| match j_type {
                    "delivery" => create_delivery_job(&format!("job{}", idx + 1), vec![loc, 0.]),
                    "pickup" => create_pickup_job(&format!("job{}", idx + 1), vec![loc, 0.]),
                    _ => unreachable!(),
                })
                .collect(),
            clustering: Some(clustering),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_vehicle_with_capacity("my_vehicle", vec![capacity])
            }],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        ..create_empty_problem()
    }
}

mod basic_vicinity_test;
mod capacity_vicinity_test;
mod profile_vicinity_test;
mod specific_vicinity_test;
