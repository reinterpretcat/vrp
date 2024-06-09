use crate::models::common::*;
use crate::models::problem::*;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

pub const DEFAULT_ACTOR_LOCATION: Location = 0;
pub const DEFAULT_ACTOR_TIME_WINDOW: TimeWindow = TimeWindow { start: 0.0, end: 1000.0 };
pub const DEFAULT_VEHICLE_COSTS: Costs =
    Costs { fixed: 0.0, per_distance: 1.0, per_driving_time: 1.0, per_waiting_time: 1.0, per_service_time: 1.0 };

pub fn test_costs() -> Costs {
    DEFAULT_VEHICLE_COSTS
}

pub fn fixed_costs() -> Costs {
    Costs { fixed: 100.0, per_distance: 1.0, per_driving_time: 1.0, per_waiting_time: 1.0, per_service_time: 1.0 }
}

pub fn empty_costs() -> Costs {
    Costs { fixed: 0.0, per_distance: 0.0, per_driving_time: 0.0, per_waiting_time: 0.0, per_service_time: 0.0 }
}

pub fn test_driver() -> Driver {
    Driver { costs: test_costs(), dimens: Default::default(), details: vec![] }
}

pub fn test_driver_with_costs(costs: Costs) -> Driver {
    Driver { costs, dimens: Default::default(), details: vec![] }
}

pub fn test_vehicle_detail() -> VehicleDetail {
    VehicleDetail {
        start: Some(VehiclePlace {
            location: 0,
            time: TimeInterval { earliest: Some(DEFAULT_ACTOR_TIME_WINDOW.start), latest: None },
        }),
        end: Some(VehiclePlace {
            location: 0,
            time: TimeInterval { earliest: None, latest: Some(DEFAULT_ACTOR_TIME_WINDOW.end) },
        }),
    }
}

pub fn test_vehicle(profile_idx: usize) -> Vehicle {
    Vehicle {
        profile: Profile::new(profile_idx, None),
        costs: test_costs(),
        dimens: Default::default(),
        details: vec![test_vehicle_detail()],
    }
}

pub fn test_ovrp_vehicle(id: &str) -> Vehicle {
    VehicleBuilder::default().id(id).details(vec![VehicleDetail { end: None, ..test_vehicle_detail() }]).build()
}

pub fn test_fleet() -> Fleet {
    FleetBuilder::default().add_driver(test_driver()).add_vehicle(test_vehicle_with_id("v1")).build()
}

pub fn test_vehicle_with_id(id: &str) -> Vehicle {
    let mut dimens = Dimensions::default();
    dimens.set_id(id);

    Vehicle { profile: Profile::default(), costs: test_costs(), dimens, details: vec![test_vehicle_detail()] }
}

pub fn get_vehicle_id(vehicle: &Vehicle) -> &String {
    vehicle.dimens.get_id().unwrap()
}

pub fn get_test_actor_from_fleet(fleet: &Fleet, vehicle_id: &str) -> Arc<Actor> {
    fleet.actors.iter().find(|actor| get_vehicle_id(&actor.vehicle) == vehicle_id).unwrap().clone()
}

pub struct VehicleBuilder(Vehicle);

impl Default for VehicleBuilder {
    fn default() -> VehicleBuilder {
        Self(test_vehicle(0))
    }
}

impl VehicleBuilder {
    pub fn id(&mut self, id: &str) -> &mut VehicleBuilder {
        self.0.dimens.set_id(id);
        self
    }

    pub fn profile(&mut self, profile: Profile) -> &mut VehicleBuilder {
        self.0.profile = profile;
        self
    }

    pub fn capacity(&mut self, capacity: i32) -> &mut VehicleBuilder {
        self.0.dimens.set_capacity(SingleDimLoad::new(capacity));
        self
    }

    pub fn costs(&mut self, costs: Costs) -> &mut VehicleBuilder {
        self.0.costs = costs;
        self
    }

    pub fn details(&mut self, details: Vec<VehicleDetail>) -> &mut VehicleBuilder {
        self.0.details = details;
        self
    }

    pub fn build(&mut self) -> Vehicle {
        std::mem::replace(&mut self.0, test_vehicle(0))
    }
}

pub type GroupKeyFn = Box<dyn Fn(&[Arc<Actor>]) -> Box<dyn Fn(&Actor) -> usize + Send + Sync>>;

#[derive(Default)]
pub struct FleetBuilder {
    drivers: Vec<Driver>,
    vehicles: Vec<Vehicle>,
    group_key_fn: Option<GroupKeyFn>,
}

impl FleetBuilder {
    pub fn add_driver(&mut self, driver: Driver) -> &mut FleetBuilder {
        self.drivers.push(driver);
        self
    }

    pub fn add_vehicle(&mut self, vehicle: Vehicle) -> &mut FleetBuilder {
        self.vehicles.push(vehicle);
        self
    }

    pub fn add_vehicles(&mut self, vehicles: Vec<Vehicle>) -> &mut FleetBuilder {
        self.vehicles.extend(vehicles);
        self
    }

    pub fn with_group_key_fn(&mut self, group_key_fn: GroupKeyFn) -> &mut FleetBuilder {
        self.group_key_fn = Some(group_key_fn);
        self
    }

    pub fn build(&mut self) -> Fleet {
        let drivers = std::mem::take(&mut self.drivers);
        let vehicles = std::mem::take(&mut self.vehicles);

        let drivers = drivers.into_iter().map(Arc::new).collect();
        let vehicles = vehicles.into_iter().map(Arc::new).collect();

        let group_key = self
            .group_key_fn
            .take()
            .unwrap_or_else(|| Box::new(|actors| Box::new(create_details_actor_groups(actors))));

        Fleet::new(drivers, vehicles, group_key)
    }
}

#[allow(clippy::type_complexity)]
pub fn create_details_actor_groups(actors: &[Arc<Actor>]) -> impl Fn(&Actor) -> usize + Send + Sync {
    let unique_type_keys: HashSet<_> = actors.iter().map(|a| a.detail.clone()).collect();

    let type_key_map: HashMap<_, _> = unique_type_keys.into_iter().zip(0_usize..).collect();

    let groups: HashMap<_, _> = actors.iter().map(|a| (a.clone(), *type_key_map.get(&a.detail).unwrap())).collect();

    move |a| *groups.get(a).unwrap()
}
