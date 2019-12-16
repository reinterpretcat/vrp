use core::construction::states::RouteContext;
use core::models::common::{Dimensions, IdDimension, ValueDimension};
use core::models::problem::{Job, Single};
use core::models::solution::Activity;
use std::sync::Arc;

pub const HAS_RELOAD_KEY: i32 = 101;
pub const MAX_TOUR_LOAD_KEY: i32 = 102;

fn as_single_job<F>(activity: &Activity, condition: F) -> Option<Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| match job.as_ref() {
        Job::Single(job) => {
            if condition(job) {
                Some(job.clone())
            } else {
                None
            }
        }
        _ => None,
    })
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

mod extra_costs;
pub use self::extra_costs::ExtraCostModule;

mod reload_capacity;
pub use self::reload_capacity::ReloadCapacityConstraintModule;

mod reachable;
pub use self::reachable::ReachableModule;

mod skills;
pub use self::skills::SkillsModule;
