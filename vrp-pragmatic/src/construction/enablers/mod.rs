//! Provides extension logic for building various VRP features.

use std::sync::Arc;
use vrp_core::models::common::Dimensions;
use vrp_core::models::problem::{Job, Single};
use vrp_core::models::solution::Route;

mod entities;
pub use self::entities::*;

mod location_fallback;
pub use self::location_fallback::*;

mod only_vehicle_activity_cost;
pub use self::only_vehicle_activity_cost::*;

mod typed_actor_group_key;
pub use self::typed_actor_group_key::*;

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

pub(crate) fn is_job_belongs_to_route(route: &Route, job: &Job) -> bool {
    job.as_single()
        .map_or(false, |job| is_correct_vehicle(route, get_vehicle_id_from_job(job), get_shift_index(&job.dimens)))
}
