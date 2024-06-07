//! Provides VRP features.

use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::prelude::*;

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
