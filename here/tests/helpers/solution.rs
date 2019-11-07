use crate::json::solution::{Activity, Schedule, Stop};

pub fn create_stop_with_activity(
    id: &str,
    activity_type: &str,
    location: (f64, f64),
    load: i32,
    time: (&str, &str),
) -> Stop {
    Stop {
        location: vec![location.0, location.1],
        time: Schedule { arrival: time.0.to_string(), departure: time.1.to_string() },
        load: vec![load],
        activities: vec![Activity {
            job_id: id.to_string(),
            activity_type: activity_type.to_string(),
            location: None,
            time: None,
            job_tag: None,
        }],
    }
}
