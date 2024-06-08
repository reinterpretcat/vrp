//! Provides VRP features.

use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;
use vrp_core::prelude::*;

pub use vrp_core::construction::features::reachable::*;

pub mod recharge;
pub use self::recharge::*;

pub mod reloads;
pub use self::reloads::*;

pub mod skills;
pub use self::skills::*;
