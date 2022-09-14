use std::sync::Arc;
use vrp_core::models::common::Dimensions;
use vrp_core::models::problem::Single;
use vrp_core::models::solution::{Activity, Route};

mod entities;
pub use self::entities::*;

mod only_vehicle_activity_cost;
pub use self::only_vehicle_activity_cost::*;

mod route_modifier;
pub use self::route_modifier::*;

mod typed_actor_group_key;
pub use self::typed_actor_group_key::*;

pub(crate) fn as_single_job<F>(activity: &Activity, condition: F) -> Option<&Arc<Single>>
where
    F: Fn(&Arc<Single>) -> bool,
{
    activity.job.as_ref().and_then(|job| if condition(job) { Some(job) } else { None })
}

pub(crate) fn get_shift_index(dimens: &Dimensions) -> usize {
    dimens.get_shift_index().expect("cannot get shift index")
}

pub(crate) fn get_vehicle_id_from_job(job: &Arc<Single>) -> &String {
    job.dimens.get_vehicle_id().expect("cannot get vehicle id")
}

pub(crate) fn is_correct_vehicle(route: &Route, target_id: &str, target_shift: usize) -> bool {
    route.actor.vehicle.dimens.get_vehicle_id().expect("cannot get vehicle id") == target_id
        && get_shift_index(&route.actor.vehicle.dimens) == target_shift
}

pub(crate) fn is_single_belongs_to_route(route: &Route, single: &Arc<Single>) -> bool {
    let vehicle_id = get_vehicle_id_from_job(single);
    let shift_index = get_shift_index(&single.dimens);

    is_correct_vehicle(route, vehicle_id, shift_index)
}
