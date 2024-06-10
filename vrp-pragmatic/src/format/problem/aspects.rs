use crate::construction::enablers::{BreakTie, JobTie, VehicleTie};
use std::marker::PhantomData;
use vrp_core::construction::features::{
    BreakAspects, BreakCandidate, BreakPolicy, CompatibilityAspects, GroupAspects, RechargeAspects,
    RechargeDistanceLimitFn, RechargeKeys, ReloadAspects,
};
use vrp_core::construction::heuristics::{RouteContext, StateKey};
use vrp_core::models::common::{CapacityDimension, Demand, DemandDimension, IdDimension, LoadOps};
use vrp_core::models::problem::{Job, Single, Vehicle};
use vrp_core::models::solution::Route;
use vrp_core::models::ViolationCode;

/// Provides a way to use break feature.
#[derive(Clone, Copy)]
pub struct PragmaticBreakAspects;

impl BreakAspects for PragmaticBreakAspects {
    fn belongs_to_route(&self, route_ctx: &RouteContext, candidate: BreakCandidate<'_>) -> bool {
        if self.is_break_job(candidate) {
            candidate.as_single().map_or(false, |single| is_correct_vehicle(route_ctx.route(), single))
        } else {
            false
        }
    }

    fn is_break_job(&self, candidate: BreakCandidate<'_>) -> bool {
        candidate
            .as_single()
            .and_then(|break_single| break_single.dimens.get_job_type())
            .map_or(false, |job_type| job_type == "break")
    }

    fn get_policy(&self, candidate: BreakCandidate<'_>) -> Option<BreakPolicy> {
        candidate.as_single().and_then(|single| single.dimens.get_break_policy())
    }
}

/// Provides a way to use compatibility feature.
#[derive(Clone)]
pub struct PragmaticCompatibilityAspects {
    state_key: StateKey,
    violation_code: ViolationCode,
}

impl PragmaticCompatibilityAspects {
    /// Creates a new instance of `PragmaticCompatibilityAspects`.
    pub fn new(state_key: StateKey, violation_code: ViolationCode) -> Self {
        Self { state_key, violation_code }
    }
}

impl CompatibilityAspects for PragmaticCompatibilityAspects {
    fn get_job_compatibility<'a>(&self, job: &'a Job) -> Option<&'a String> {
        job.dimens().get_job_compatibility()
    }

    fn get_state_key(&self) -> StateKey {
        self.state_key
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
    }
}

/// Provides a way to use the group feature.
#[derive(Clone)]
pub struct PragmaticGroupAspects {
    state_key: StateKey,
    violation_code: ViolationCode,
}

impl PragmaticGroupAspects {
    /// Creates a new instance of `PragmaticGroupAspects`.
    pub fn new(state_key: StateKey, violation_code: ViolationCode) -> Self {
        Self { state_key, violation_code }
    }
}

impl GroupAspects for PragmaticGroupAspects {
    fn get_job_group<'a>(&self, job: &'a Job) -> Option<&'a String> {
        job.dimens().get_job_group()
    }

    fn get_state_key(&self) -> StateKey {
        self.state_key
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
    }
}

/// Provides a way to use recharge feature.
#[derive(Clone)]
pub struct PragmaticRechargeAspects {
    recharge_keys: RechargeKeys,
    violation_code: ViolationCode,
    distance_limit_fn: RechargeDistanceLimitFn,
}

impl PragmaticRechargeAspects {
    /// Creates a new instance of `PragmaticRechargeAspects`.
    pub fn new(
        recharge_keys: RechargeKeys,
        violation_code: ViolationCode,
        distance_limit_fn: RechargeDistanceLimitFn,
    ) -> Self {
        Self { recharge_keys, violation_code, distance_limit_fn }
    }
}

impl RechargeAspects for PragmaticRechargeAspects {
    fn belongs_to_route(&self, route: &Route, job: &Job) -> bool {
        job.as_single()
            .map_or(false, |single| self.is_recharge_single(single.as_ref()) && is_correct_vehicle(route, single))
    }

    fn is_recharge_single(&self, single: &Single) -> bool {
        single.dimens.get_job_type().map_or(false, |job_type| job_type == "recharge")
    }

    fn get_state_keys(&self) -> &RechargeKeys {
        &self.recharge_keys
    }

    fn get_distance_limit_fn(&self) -> RechargeDistanceLimitFn {
        self.distance_limit_fn.clone()
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
    }
}

/// Provides a way to use reload feature.
#[derive(Clone, Default)]
pub struct PragmaticReloadAspects<T> {
    phantom: PhantomData<T>,
}

impl<T: LoadOps> ReloadAspects<T> for PragmaticReloadAspects<T> {
    fn belongs_to_route(&self, route: &Route, job: &Job) -> bool {
        job.as_single()
            .map_or(false, |single| self.is_reload_single(single.as_ref()) && is_correct_vehicle(route, single))
    }

    fn is_reload_single(&self, single: &Single) -> bool {
        single.dimens.get_job_type().map_or(false, |job_type| job_type == "reload")
    }

    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T> {
        vehicle.dimens.get_capacity()
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>> {
        single.dimens.get_demand()
    }
}

fn is_correct_vehicle(route: &Route, single: &Single) -> bool {
    let job_vehicle_id = single.dimens.get_vehicle_id();
    let job_shift_idx = single.dimens.get_shift_index();

    let vehicle = &route.actor.vehicle;
    let vehicle_id = vehicle.dimens.get_id();
    let vehicle_shift_idx = vehicle.dimens.get_shift_index();

    job_vehicle_id == vehicle_id && job_shift_idx == vehicle_shift_idx
}
