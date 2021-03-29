use crate::construction::constraints::*;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use crate::models::common::{Demand, SingleDimLoad};
use std::sync::Arc;

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

pub fn create_constraint_pipeline_with_module(module: Box<dyn ConstraintModule + Send + Sync>) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(module);
    constraint
}

pub fn create_constraint_pipeline_with_transport() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Box::new(TransportConstraintModule::new(
        TestTransportCost::new_shared(),
        Arc::new(TestActivityCost::default()),
        Arc::new(|_| (None, None)),
        1,
        2,
        3,
    )))
}

pub fn create_constraint_pipeline_with_simple_capacity() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Box::new(CapacityConstraintModule::<SingleDimLoad>::new(
        TestTransportCost::new_shared(),
        2,
    )))
}
