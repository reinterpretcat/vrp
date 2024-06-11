use crate::construction::features::{CapacityAspects, CapacityStateKeys};
use crate::models::common::{Demand, LoadOps, MultiDimLoad, SingleDimLoad, ValueDimension};
use crate::models::problem::{Single, Vehicle};
use crate::models::ViolationCode;
use std::marker::PhantomData;

pub fn create_simple_demand(size: i32) -> Demand<SingleDimLoad> {
    if size > 0 {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::new(size), SingleDimLoad::default()),
            delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
        }
    } else {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
            delivery: (SingleDimLoad::new(-size), SingleDimLoad::default()),
        }
    }
}

pub fn create_simple_dynamic_demand(size: i32) -> Demand<SingleDimLoad> {
    if size > 0 {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::new(size)),
            delivery: (SingleDimLoad::default(), SingleDimLoad::default()),
        }
    } else {
        Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
            delivery: (SingleDimLoad::default(), SingleDimLoad::new(-size)),
        }
    }
}

pub fn single_demand_as_multi(pickup: (i32, i32), delivery: (i32, i32)) -> Demand<MultiDimLoad> {
    let make = |value| {
        if value == 0 {
            MultiDimLoad::default()
        } else {
            MultiDimLoad::new(vec![value])
        }
    };

    Demand { pickup: (make(pickup.0), make(pickup.1)), delivery: (make(delivery.0), make(delivery.1)) }
}

/// Creates test capacity aspects.
pub struct TestCapacityAspects<T: LoadOps> {
    capacity_keys: CapacityStateKeys,
    violation_code: ViolationCode,
    phantom: PhantomData<T>,
}

impl<T: LoadOps> TestCapacityAspects<T> {
    /// Creates a new instance of `TestCapacityAspects`.
    pub fn new(capacity_keys: CapacityStateKeys, violation_code: ViolationCode) -> Self {
        Self { capacity_keys, violation_code, phantom: Default::default() }
    }
}

impl<T: LoadOps> CapacityAspects<T> for TestCapacityAspects<T> {
    fn get_capacity<'a>(&self, vehicle: &'a Vehicle) -> Option<&'a T> {
        vehicle.dimens.get_value("capacity")
    }

    fn get_demand<'a>(&self, single: &'a Single) -> Option<&'a Demand<T>> {
        single.dimens.get_value("demand")
    }

    fn set_demand(&self, single: &mut Single, demand: Demand<T>) {
        single.dimens.set_value("demand", demand);
    }

    fn get_state_keys(&self) -> &CapacityStateKeys {
        &self.capacity_keys
    }

    fn get_violation_code(&self) -> ViolationCode {
        self.violation_code
    }
}
