//! Specifies objective functions.

use crate::construction::heuristics::InsertionContext;
use std::cmp::Ordering;

pub const BALANCE_MAX_LOAD_KEY: i32 = 20;
pub const BALANCE_ACTIVITY_KEY: i32 = 21;
pub const BALANCE_DISTANCE_KEY: i32 = 22;
pub const BALANCE_DURATION_KEY: i32 = 23;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod work_balance;
pub use self::work_balance::WorkBalance;
