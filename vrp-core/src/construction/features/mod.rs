//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::*;
use crate::models::*;
use rosomaxa::prelude::*;
use std::slice::Iter;
use std::sync::Arc;

mod capacity;
pub use self::capacity::*;

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

/// A key which tracks latest arrival.
pub const LATEST_ARRIVAL_KEY: StateKey = 1;
/// A key which tracks waiting time.
pub const WAITING_KEY: StateKey = 2;
/// A key which tracks total distance.
pub const TOTAL_DISTANCE_KEY: StateKey = 3;
/// A key which tracks total duration.
pub const TOTAL_DURATION_KEY: StateKey = 4;
/// A key which tracks global duration limit.
pub const LIMIT_DURATION_KEY: StateKey = 5;

/// A key which tracks current vehicle capacity.
pub const CURRENT_CAPACITY_KEY: StateKey = 11;
/// A key which tracks maximum vehicle capacity ahead in route.
pub const MAX_FUTURE_CAPACITY_KEY: StateKey = 12;
/// A key which tracks maximum capacity backward in route.
pub const MAX_PAST_CAPACITY_KEY: StateKey = 13;
/// A key which tracks reload intervals.
pub const RELOAD_INTERVALS_KEY: StateKey = 14;
/// A key which tracks max load in tour.
pub const MAX_LOAD_KEY: StateKey = 15;
/// A key which tracks recharge intervals.
pub const RECHARGE_INTERVALS_KEY: StateKey = 16;
/// A key to track recharge distance.
pub const RECHARGE_DISTANCE_KEY: StateKey = 17;

/// A key for balancing max load.
pub const BALANCE_MAX_LOAD_KEY: StateKey = 20;
/// A key for balancing activities.
pub const BALANCE_ACTIVITY_KEY: StateKey = 21;
/// A key for balancing distance.
pub const BALANCE_DISTANCE_KEY: StateKey = 22;
/// A key for balancing duration.
pub const BALANCE_DURATION_KEY: StateKey = 23;
