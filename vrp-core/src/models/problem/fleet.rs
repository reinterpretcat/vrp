#[cfg(test)]
#[path = "../../../tests/unit/models/problem/fleet_test.rs"]
mod fleet_test;

use crate::models::common::{Dimensions, Location, Profile, TimeWindow};
use std::cmp::Ordering::Less;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Represents operating costs for driver and vehicle.
#[derive(Clone, Debug)]
pub struct Costs {
    /// A fixed cost to use an actor.
    pub fixed: f64,
    /// Cost per distance unit.
    pub per_distance: f64,
    /// Cost per driving time unit.
    pub per_driving_time: f64,
    /// Cost per waiting time unit.
    pub per_waiting_time: f64,
    /// Cost per service time unit.
    pub per_service_time: f64,
}

/// Represents driver detail.
pub struct DriverDetail {
    /// Time windows when driver can work.
    pub time: Option<TimeWindow>,
}

/// Represents a driver, person who drives Vehicle.
/// Introduced to allow the following scenarios:
/// * reuse vehicle multiple times with different drivers
/// * solve best driver-vehicle match problem.
/// NOTE: At the moment, it is not used.
pub struct Driver {
    /// Specifies operating costs for driver.
    pub costs: Costs,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
    /// Specifies driver details.
    pub details: Vec<DriverDetail>,
}

/// Represents a vehicle detail.
#[derive(Clone, Debug)]
pub struct VehicleDetail {
    /// Location where vehicle starts.
    pub start: Option<Location>,
    /// Location where vehicle ends.
    pub end: Option<Location>,
    /// Time windows when driver can work.
    pub time: Option<TimeWindow>,
}

/// Represents a vehicle.
pub struct Vehicle {
    pub profile: Profile,
    /// Specifies operating costs for vehicle.
    pub costs: Costs,
    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,
    /// Specifies vehicle details.
    pub details: Vec<VehicleDetail>,
}

/// Represents actor detail.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ActorDetail {
    /// Location where actor starts.
    pub start: Option<Location>,

    /// Location where actor ends.
    pub end: Option<Location>,

    /// Time windows when actor can work.
    pub time: TimeWindow,
}

/// Represents an actor.
pub struct Actor {
    /// A vehicle associated within actor.
    pub vehicle: Arc<Vehicle>,

    /// A driver associated within actor.
    pub driver: Arc<Driver>,

    /// Specifies actor detail.
    pub detail: ActorDetail,
}

/// Represents available resources to serve jobs.
pub struct Fleet {
    pub drivers: Vec<Arc<Driver>>,
    pub vehicles: Vec<Arc<Vehicle>>,
    pub profiles: Vec<Profile>,
    pub actors: Vec<Arc<Actor>>,
}

impl Fleet {
    /// Creates a new fleet.
    pub fn new(drivers: Vec<Driver>, vehicles: Vec<Vehicle>) -> Fleet {
        // TODO we should also consider multiple drivers to support smart vehicle-driver assignment.
        assert_eq!(drivers.len(), 1);
        assert!(!vehicles.is_empty());

        let profiles: HashSet<Profile> = vehicles.iter().map(|v| v.profile).collect();
        let mut profiles: Vec<Profile> = profiles.into_iter().map(|p| p).collect();
        profiles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Less));

        let drivers: Vec<Arc<Driver>> = drivers.into_iter().map(Arc::new).collect();
        let vehicles: Vec<Arc<Vehicle>> = vehicles.into_iter().map(Arc::new).collect();

        let mut actors: Vec<Arc<Actor>> = Default::default();
        vehicles.iter().for_each(|vehicle| {
            vehicle.details.iter().for_each(|detail| {
                actors.push(Arc::new(Actor {
                    vehicle: vehicle.clone(),
                    driver: drivers.first().unwrap().clone(),
                    detail: ActorDetail {
                        start: detail.start,
                        end: detail.end,
                        time: detail.time.clone().unwrap_or(TimeWindow { start: 0.0, end: std::f64::MAX }),
                    },
                }));
            });
        });

        Fleet { drivers, vehicles, actors, profiles }
    }
}

impl Hash for Costs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let fixed: i64 = unsafe { std::mem::transmute(self.fixed) };
        let per_distance: i64 = unsafe { std::mem::transmute(self.per_distance) };
        let per_driving_time: i64 = unsafe { std::mem::transmute(self.per_driving_time) };
        let per_service_time: i64 = unsafe { std::mem::transmute(self.per_service_time) };
        let per_waiting_time: i64 = unsafe { std::mem::transmute(self.per_waiting_time) };

        fixed.hash(state);
        per_distance.hash(state);
        per_driving_time.hash(state);
        per_service_time.hash(state);
        per_waiting_time.hash(state);
    }
}

impl Eq for Costs {}

impl PartialEq for Costs {
    fn eq(&self, other: &Self) -> bool {
        self.fixed == other.fixed
            && self.per_distance == other.per_distance
            && self.per_driving_time == other.per_driving_time
            && self.per_service_time == other.per_service_time
            && self.per_waiting_time == other.per_waiting_time
    }
}
