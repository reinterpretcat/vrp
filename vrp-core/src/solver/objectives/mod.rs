//! The objective module specifies various objective functions for solving Vehicle Routing Problem.

use crate::construction::heuristics::InsertionContext;
use std::cmp::Ordering;

mod generic_value;
pub(crate) use self::generic_value::GenericValue;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport_cost;
pub use self::total_transport_cost::TotalTransportCost;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod work_balance;
pub use self::work_balance::WorkBalance;
