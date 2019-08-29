use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::models::common::{Dimensions, Location, Profile, TimeWindow};
use crate::models::problem::{Costs, Driver, Fleet, Vehicle, VehicleDetail};

pub const DEFAULT_ACTOR_LOCATION: Location = 0;
pub const DEFAULT_ACTOR_TIME_WINDOW: TimeWindow = TimeWindow {
    start: 0.0,
    end: 1000.0,
};
pub const DEFAULT_VEHICLE_COSTS: Costs = Costs {
    fixed: 100.0,
    per_distance: 1.0,
    per_driving_time: 1.0,
    per_waiting_time: 1.0,
    per_service_time: 1.0,
};

pub fn test_costs() -> Costs {
    DEFAULT_VEHICLE_COSTS
}

pub fn test_driver() -> Driver {
    Driver {
        costs: test_costs(),
        dimens: Default::default(),
        details: vec![],
    }
}

pub fn test_vehicle_detail() -> VehicleDetail {
    VehicleDetail {
        start: Some(0),
        end: Some(0),
        time: Some(DEFAULT_ACTOR_TIME_WINDOW),
    }
}

pub fn test_vehicle(profile: i32) -> Vehicle {
    Vehicle {
        profile,
        costs: test_costs(),
        dimens: Default::default(),
        details: vec![test_vehicle_detail()],
    }
}

pub struct VehicleBuilder {
    vehicle: Vehicle,
}

impl VehicleBuilder {
    pub fn new() -> VehicleBuilder {
        VehicleBuilder {
            vehicle: test_vehicle(DEFAULT_PROFILE),
        }
    }

    pub fn id(&mut self, id: &str) -> &mut VehicleBuilder {
        self.vehicle
            .dimens
            .insert("id".to_string(), Box::new(id.to_string()));
        self
    }

    pub fn profile(&mut self, profile: Profile) -> &mut VehicleBuilder {
        self.vehicle.profile = profile;
        self
    }

    pub fn costs(&mut self, costs: Costs) -> &mut VehicleBuilder {
        self.vehicle.costs = costs;
        self
    }

    pub fn details(&mut self, details: Vec<VehicleDetail>) -> &mut VehicleBuilder {
        self.vehicle.details = details;
        self
    }

    pub fn dimens(&mut self, dimens: Dimensions) -> &mut VehicleBuilder {
        self.vehicle.dimens = dimens;
        self
    }

    pub fn build(&mut self) -> Vehicle {
        std::mem::replace(&mut self.vehicle, test_vehicle(0))
    }
}

pub struct FleetBuilder {
    drivers: Vec<Driver>,
    vehicles: Vec<Vehicle>,
}

impl FleetBuilder {
    pub fn new() -> FleetBuilder {
        FleetBuilder {
            drivers: Default::default(),
            vehicles: Default::default(),
        }
    }

    pub fn add_driver(&mut self, driver: Driver) -> &mut FleetBuilder {
        self.drivers.push(driver);
        self
    }

    pub fn add_vehicle(&mut self, vehicle: Vehicle) -> &mut FleetBuilder {
        self.vehicles.push(vehicle);
        self
    }

    pub fn build(self) -> Fleet {
        Fleet::new(self.drivers, self.vehicles)
    }
}
