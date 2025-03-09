use crate::models::common::{Demand, MultiDimLoad, SingleDimLoad};

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
        if value == 0 { MultiDimLoad::default() } else { MultiDimLoad::new(vec![value]) }
    };

    Demand { pickup: (make(pickup.0), make(pickup.1)), delivery: (make(delivery.0), make(delivery.1)) }
}
