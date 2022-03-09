//! The objective module specifies various objective functions for solving Vehicle Routing Problem.

use crate::construction::heuristics::InsertionContext;
use std::cmp::Ordering;

mod generic_value;
pub use self::generic_value::*;

mod minimize_arrival_time;
pub use self::minimize_arrival_time::*;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport;
pub use self::total_transport::*;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod total_value;
pub use self::total_value::*;

mod tour_order;
pub use self::tour_order::*;

mod work_balance;
pub use self::work_balance::WorkBalance;
