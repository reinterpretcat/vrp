//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::*;
use rosomaxa::prelude::*;
use std::slice::Iter;
use std::sync::Arc;

mod breaks;
pub use self::breaks::*;

mod capacity;
pub use self::capacity::*;

mod compatibility;
pub use self::compatibility::*;

mod fast_service;
pub use self::fast_service::*;

mod fleet_usage;
pub use self::fleet_usage::*;

mod locked_jobs;
pub use self::locked_jobs::*;

mod minimize_unassigned;
pub use self::minimize_unassigned::*;

mod shared_resource;
pub use self::shared_resource::*;

mod total_value;
pub use self::total_value::*;

mod tour_compactness;
pub use self::tour_compactness::*;

mod tour_limits;
pub use self::tour_limits::*;

mod tour_order;
pub use self::tour_order::*;

mod transport;
pub use self::transport::*;

mod work_balance;
pub use self::work_balance::*;
