use crate::models::common::{Demand, SingleDimLoad};

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
