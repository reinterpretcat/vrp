use crate::construction::constraints::{CapacityConstraintModule, ConstraintPipeline, TimingConstraintModule};
use crate::helpers::models::problem::{TestActivityCost, TestTransportCost};
use std::sync::Arc;

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
