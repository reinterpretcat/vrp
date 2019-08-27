use crate::models::problem::{Costs, Driver, Vehicle};

pub fn test_costs() -> Costs {
    Costs {
        fixed: 0.0,
        per_distance: 1.0,
        per_driving_time: 1.0,
        per_waiting_time: 1.0,
        per_service_time: 1.0,
    }
}

pub fn test_driver() -> Driver {
    Driver {
        costs: test_costs(),
        dimens: Default::default(),
        details: vec![],
    }
}

pub fn test_vehicle(profile: i32) -> Vehicle {
    Vehicle {
        profile,
        costs: test_costs(),
        dimens: Default::default(),
        details: vec![],
    }
}
