use crate::format::{BreakTie, JobTie, VehicleTie};
use hashbrown::HashSet;
use std::marker::PhantomData;
use vrp_core::construction::features::*;
use vrp_core::construction::heuristics::{RouteContext, StateKey};
use vrp_core::models::common::{
    CapacityDimension, Demand, DemandDimension, DemandType, IdDimension, LoadOps, MultiDimLoad, SingleDimLoad,
};
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

/// Provides a way to use capacity feature.
pub struct PragmaticCapacityAspects<T: LoadOps> {
    state_keys: CapacityStateKeys,
    violation_code: ViolationCode,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> PragmaticCapacityAspects<T> {
    /// Creates a new instance of `PragmaticCapacityAspects`.
    pub fn new(state_keys: CapacityStateKeys, violation_code: ViolationCode) -> Self {
        Self { state_keys, violation_code, phantom: Default::default() }
    }
}

impl<T: LoadOps> CapacityAspects<T> for PragmaticCapacityAspects<T> {
    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T> {
        vehicle.dimens.get_capacity()
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>> {
        single.dimens.get_demand()
    }

    fn set_demand(&self, single: &mut Single, demand: Demand<T>) {
        single.dimens.set_demand(demand);
    }

    fn get_state_keys(&self) -> &CapacityStateKeys {
        &self.state_keys
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
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

pub struct PragmaticFastServiceAspects {
    state_key: StateKey,
}

impl PragmaticFastServiceAspects {
    /// Creates a new instance of `PragmaticFastServiceAspects`.
    pub fn new(state_key: StateKey) -> Self {
        Self { state_key }
    }
}

impl FastServiceAspects for PragmaticFastServiceAspects {
    fn get_state_key(&self) -> StateKey {
        self.state_key
    }

    fn get_demand_type(&self, single: &Single) -> Option<DemandType> {
        let demand_single: Option<&Demand<SingleDimLoad>> = single.dimens.get_demand();
        let demand_multi: Option<&Demand<MultiDimLoad>> = single.dimens.get_demand();

        demand_single.map(|d| d.get_type()).or_else(|| demand_multi.map(|d| d.get_type()))
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

#[derive(Clone)]
pub struct PragmaticJobSkillsAspects {
    violation_code: ViolationCode,
}

impl PragmaticJobSkillsAspects {
    /// Creates a new instance of `PragmaticJobSkillsAspects`.
    pub fn new(violation_code: ViolationCode) -> Self {
        Self { violation_code }
    }
}

impl JobSkillsAspects for PragmaticJobSkillsAspects {
    fn get_job_skills<'a>(&self, job: &'a Job) -> Option<&'a JobSkills> {
        job.dimens().get_job_skills()
    }

    fn get_vehicle_skills<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a HashSet<String>> {
        vehicle.dimens.get_vehicle_skills()
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
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
