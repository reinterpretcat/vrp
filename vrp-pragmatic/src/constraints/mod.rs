//! Contains implementation of extra constraints.

use crate::extensions::VehicleTie;
use std::sync::Arc;
use vrp_core::construction::heuristics::RouteContext;
use vrp_core::models::common::Dimensions;
use vrp_core::models::problem::Single;
use vrp_core::models::solution::{Activity, Route};

/// A key which tracks job group state.
pub const GROUP_KEY: i32 = 1000;
/// A key which tracks compatibility key.
pub const COMPATIBILITY_KEY: i32 = 1001;

/// A key which tracks total value.
pub const TOTAL_VALUE_KEY: i32 = 1002;
/// A key which tracks tour order state.
pub const TOUR_ORDER_KEY: i32 = 1003;

/// A key which tracks area value state.
pub const AREA_VALUE_KEY: i32 = 1004;
/// A key which tracks area order state.
pub const AREA_ORDER_KEY: i32 = 1005;

/// A key which tracks reload resource consumption state.
pub const RELOAD_RESOURCE_KEY: i32 = 1006;

fn as_single_job<F>(activity: &Activity, condition: F) -> Option<&Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| if condition(job) { Some(job) } else { None })
}

fn get_shift_index(dimens: &Dimensions) -> usize {
    dimens.get_shift_index().expect("cannot get shift index")
}

fn get_vehicle_id_from_job(job: &Arc<Single>) -> &String {
    job.dimens.get_vehicle_id().expect("cannot get vehicle id")
}

fn is_correct_vehicle(route: &Route, target_id: &str, target_shift: usize) -> bool {
    route.actor.vehicle.dimens.get_vehicle_id().expect("cannot get vehicle id") == target_id
        && get_shift_index(&route.actor.vehicle.dimens) == target_shift
}

fn is_single_belongs_to_route(ctx: &RouteContext, single: &Arc<Single>) -> bool {
    let vehicle_id = get_vehicle_id_from_job(single);
    let shift_index = get_shift_index(&single.dimens);

    is_correct_vehicle(&ctx.route, vehicle_id, shift_index)
}

mod areas;
pub use self::areas::AreaModule;

mod breaks;
pub use self::breaks::{BreakModule, BreakPolicy};

mod compatibility;
pub use self::compatibility::CompatibilityModule;

mod dispatch;
pub use self::dispatch::DispatchModule;

mod groups;
pub use self::groups::GroupModule;

mod reloads;
pub use self::reloads::*;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::JobSkills;
pub use self::skills::SkillsModule;
