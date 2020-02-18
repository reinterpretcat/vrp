//! This module provides default strategies.

use super::*;

pub fn default_job_single_day_time_windows() -> impl Strategy<Value = Vec<Vec<String>>> {
    generate_multiple_time_windows_fixed(
        "2020-07-04T00:00:00Z",
        vec![from_hours(8), from_hours(14)],
        vec![from_hours(2), from_hours(4)],
        1..3,
    )
}
