//! Specifies objective functions.

use crate::construction::heuristics::InsertionContext;
use std::cmp::Ordering;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod work_balance;
pub use self::work_balance::WorkBalance;
