//! The objective module specifies various objective functions for solving Vehicle Routing Problem.

use crate::construction::heuristics::InsertionContext;
use std::cmp::Ordering;

mod generic_value;
pub(crate) use self::generic_value::GenericValue;

mod total_routes;
pub use self::total_routes::TotalRoutes;

mod total_transport;
pub use self::total_transport::*;

mod total_unassigned_jobs;
pub use self::total_unassigned_jobs::TotalUnassignedJobs;

mod total_value;
pub use self::total_value::TotalValue;

mod tour_order;
pub use self::tour_order::TourOrder;

mod work_balance;
pub use self::work_balance::WorkBalance;
