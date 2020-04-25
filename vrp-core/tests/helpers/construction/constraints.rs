use crate::construction::constraints::*;
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use std::sync::Arc;

pub fn create_simple_demand(size: i32) -> Demand<i32> {
    if size > 0 {
        Demand::<i32> { pickup: (size, 0), delivery: (0, 0) }
    } else {
        Demand::<i32> { pickup: (0, 0), delivery: (-size, 0) }
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
    create_constraint_pipeline_with_module(Box::new(CapacityConstraintModule::<i32>::new(2)))
}
