use std::slice::Iter;
use std::sync::Arc;
use vrp_core::construction::heuristics::*;
use vrp_core::models::common::*;
use vrp_core::models::problem::Job;
use vrp_core::rosomaxa::prelude::*;

pub mod breaks;
pub mod compatibility;
pub mod dispatch;
pub mod groups;
pub mod reachable;
pub mod reloads;
pub mod skills;
