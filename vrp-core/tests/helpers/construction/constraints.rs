use crate::construction::constraints::*;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use crate::models::common::{Demand, SingleDimCapacity};
use std::sync::Arc;

pub fn create_simple_demand(size: i32) -> Demand<SingleDimCapacity> {
    if size > 0 {
        Demand::<SingleDimCapacity> {
            pickup: (SingleDimCapacity::new(size), SingleDimCapacity::default()),
            delivery: (SingleDimCapacity::default(), SingleDimCapacity::default()),
        }
    } else {
        Demand::<SingleDimCapacity> {
            pickup: (SingleDimCapacity::default(), SingleDimCapacity::default()),
            delivery: (SingleDimCapacity::new(-size), SingleDimCapacity::default()),
        }
    }
}

pub fn create_constraint_pipeline_with_module(module: Box<dyn ConstraintModule + Send + Sync>) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(module);
    constraint
}

pub fn create_constraint_pipeline_with_transport() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Box::new(TransportConstraintModule::new(
        Arc::new(TestActivityCost::default()),
        TestTransportCost::new_shared(),
        Arc::new(|_| (None, None)),
        1,
        2,
        3,
    )))
}

pub fn create_constraint_pipeline_with_simple_capacity() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Box::new(CapacityConstraintModule::<SingleDimCapacity>::new(2)))
}
