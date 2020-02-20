use std::sync::Arc;
use vrp_core::construction::states::RouteContext;
use vrp_core::models::common::{Dimensions, IdDimension, ValueDimension};
use vrp_core::models::problem::Single;
use vrp_core::models::solution::Activity;

pub const RELOAD_INTERVALS: i32 = 102;

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

fn is_correct_vehicle(rc: &RouteContext, target_id: &String, target_shift: usize) -> bool {
    rc.route.actor.vehicle.dimens.get_id().unwrap() == target_id
        && get_shift_index(&rc.route.actor.vehicle.dimens) == target_shift
}

mod breaks;
pub use self::breaks::BreakModule;

mod even_dist;
pub use self::even_dist::EvenDistributionModule;

mod reload_capacity;
pub use self::reload_capacity::reload_intervals;
pub use self::reload_capacity::ReloadCapacityConstraintModule;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::SkillsModule;
