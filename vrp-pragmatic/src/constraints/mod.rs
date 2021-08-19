//! Contains implementation of extra constraints.

use std::sync::Arc;
use vrp_core::construction::heuristics::RouteContext;
use vrp_core::models::common::{Dimensions, IdDimension, ValueDimension};
use vrp_core::models::problem::Single;
use vrp_core::models::solution::{Activity, Route};

/// A key which tracks job group state.
pub const JOB_GROUP_KEY: i32 = 1000;

fn as_single_job<F>(activity: &Activity, condition: F) -> Option<&Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| if condition(job) { Some(job) } else { None })
}

fn get_shift_index(dimens: &Dimensions) -> usize {
    *dimens.get_value::<usize>("shift_index").unwrap()
}

fn get_vehicle_id_from_job(job: &Arc<Single>) -> Option<&String> {
    job.dimens.get_value::<String>("vehicle_id")
}

fn is_correct_vehicle(route: &Route, target_id: &str, target_shift: usize) -> bool {
    route.actor.vehicle.dimens.get_id().unwrap() == target_id
        && get_shift_index(&route.actor.vehicle.dimens) == target_shift
}

fn is_single_belongs_to_route(ctx: &RouteContext, single: &Arc<Single>) -> bool {
    let vehicle_id = get_vehicle_id_from_job(single).unwrap();
    let shift_index = get_shift_index(&single.dimens);

    is_correct_vehicle(&ctx.route, vehicle_id, shift_index)
}

mod breaks;
pub use self::breaks::BreakModule;

mod dispatch;
pub use self::dispatch::DispatchModule;

mod groups;
pub use self::groups::GroupModule;

mod reloads;
pub use self::reloads::ReloadMultiTrip;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::JobSkills;
pub use self::skills::SkillsModule;
