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

pub fn create_constraint_pipeline_with_modules(
    modules: Vec<Arc<dyn ConstraintModule + Send + Sync>>,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    modules.into_iter().for_each(|module| {
        constraint.add_module(module);
    });
    constraint
}

pub fn create_constraint_pipeline_with_module(module: Arc<dyn ConstraintModule + Send + Sync>) -> ConstraintPipeline {
    create_constraint_pipeline_with_modules(vec![module])
}

pub fn create_constraint_pipeline_with_transport() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Arc::new(TransportConstraintModule::new(
        TestTransportCost::new_shared(),
        TestActivityCost::new_shared(),
        1,
    )))
}

pub fn create_constraint_pipeline_with_simple_capacity() -> ConstraintPipeline {
    create_constraint_pipeline_with_module(Arc::new(CapacityConstraintModule::<SingleDimLoad>::new(2)))
}
