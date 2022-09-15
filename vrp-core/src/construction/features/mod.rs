//! Provides extensions to build vrp variants as features.

use crate::construction::heuristics::*;
use crate::models::common::*;
use crate::models::problem::Job;
use rosomaxa::prelude::*;
use std::slice::Iter;
use std::sync::Arc;

pub mod capacity;
pub mod fleet_usage;
pub mod locked_jobs;
pub mod minimize_unassigned;
pub mod shared_resource;
pub mod total_value;
pub mod tour_limits;
pub mod tour_order;
pub mod transport;
pub mod work_balance;

// TODO move state keys here

/// Keys for balancing objectives.
const BALANCE_MAX_LOAD_KEY: i32 = 20;
const BALANCE_ACTIVITY_KEY: i32 = 21;
const BALANCE_DISTANCE_KEY: i32 = 22;
const BALANCE_DURATION_KEY: i32 = 23;
