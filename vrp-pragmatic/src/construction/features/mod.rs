//! Provides VRP features.

use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::prelude::*;

/// A key which tracks job group state.
pub const GROUP_KEY: StateKey = 1000;
/// A key which tracks compatibility key.
pub const COMPATIBILITY_KEY: StateKey = 1001;
/// A key which tracks tour order state.
pub const TOUR_ORDER_KEY: StateKey = 1002;
/// A key which tracks reload resource consumption state.
pub const RELOAD_RESOURCE_KEY: StateKey = 1003;
/// A key which tracks tour compactness state.
pub const TOUR_COMPACTNESS_KEY: StateKey = 1004;
/// A key to track fast service feature state.
pub const FAST_SERVICE_KEY: StateKey = 1005;

mod breaks;
pub use self::breaks::*;

pub mod compatibility;
pub use self::compatibility::*;

pub mod groups;
pub use self::groups::*;

pub mod reachable;
pub use self::reachable::*;

pub mod recharge;
pub use self::recharge::*;

pub mod reloads;
pub use self::reloads::*;

pub mod skills;
pub use self::skills::*;
