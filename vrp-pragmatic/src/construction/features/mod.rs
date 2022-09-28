//! Provides VRP features.

use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;

/// A key which tracks job group state.
pub const GROUP_KEY: i32 = 1000;
/// A key which tracks compatibility key.
pub const COMPATIBILITY_KEY: i32 = 1001;
/// A key which tracks tour order state.
pub const TOUR_ORDER_KEY: i32 = 1002;
/// A key which tracks reload resource consumption state.
pub const RELOAD_RESOURCE_KEY: i32 = 1003;

mod breaks;
pub use self::breaks::*;

pub mod compatibility;
pub use self::compatibility::*;

pub mod dispatch;
pub use self::dispatch::*;

pub mod groups;
pub use self::groups::*;

pub mod reachable;
pub use self::reachable::*;

pub mod reloads;
pub use self::reloads::*;

pub mod skills;
pub use self::skills::*;
