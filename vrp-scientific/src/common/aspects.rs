use vrp_core::construction::features::{CapacityAspects, CapacityStateKeys};
use vrp_core::models::common::{Demand, SingleDimLoad, ValueDimension};
use vrp_core::models::problem::{Single, Vehicle};
use vrp_core::models::ViolationCode;

/// Provides a way to use capacity feature.
pub struct ScientificCapacityAspects {
    state_keys: CapacityStateKeys,
    violation_code: ViolationCode,
}

impl ScientificCapacityAspects {
    /// Creates a new instance of `ScientificCapacityAspects`.
    pub fn new(state_keys: CapacityStateKeys, violation_code: ViolationCode) -> Self {
        Self { state_keys, violation_code }
    }
}

impl CapacityAspects<SingleDimLoad> for ScientificCapacityAspects {
    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a SingleDimLoad> {
        vehicle.dimens.get_value("capacity")
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<SingleDimLoad>> {
        single.dimens.get_value("demand")
    }

    fn set_demand(&self, single: &mut Single, demand: Demand<SingleDimLoad>) {
        single.dimens.set_value("demand", demand);
    }

    fn get_state_keys(&self) -> &CapacityStateKeys {
        &self.state_keys
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
    }
}
