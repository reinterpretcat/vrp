use crate::construction::constraints::{CapacityConstraintModule, ConstraintPipeline, Demand, TimingConstraintModule};
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use std::sync::Arc;

pub fn create_simple_demand(size: i32) -> Demand<i32> {
    if size > 0 {
        Demand::<i32> { pickup: (size, 0), delivery: (0, 0) }
    } else {
        Demand::<i32> { pickup: (0, 0), delivery: (-size, 0) }
    }
}

pub fn create_constraint_pipeline_with_timing() -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(TimingConstraintModule::new(
        Arc::new(TestActivityCost::new()),
        Arc::new(TestTransportCost::new()),
        1,
    )));
    constraint
}

pub fn create_constraint_pipeline_with_simple_capacity() -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));
    constraint
}

pub fn create_constraint_pipeline() -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::new();
    constraint.add_module(Box::new(TimingConstraintModule::new(
        Arc::new(TestActivityCost::new()),
        Arc::new(TestTransportCost::new()),
        1,
    )));
    constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));
    constraint
}
