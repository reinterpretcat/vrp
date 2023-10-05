#[cfg(test)]
#[path = "../../../tests/unit/models/problem/fleet_test.rs"]
mod fleet_test;

use crate::models::common::*;
use crate::utils::short_type_name;
use hashbrown::{HashMap, HashSet};
use std::cmp::Ordering::Less;
use std::fmt::{Debug, Formatter};
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

/// Represents driver detail (reserved for future use).
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct DriverDetail {}

/// Represents a driver, person who drives Vehicle. Reserved for future usage, e.g. to allow
/// reuse same vehicle more than once at different times.
pub struct Driver {
    /// Specifies operating costs for driver.
    pub costs: Costs,

    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,

    /// Specifies driver details.
    pub details: Vec<DriverDetail>,
}

/// Specifies a vehicle place.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct VehiclePlace {
    /// Location of a place.
    pub location: Location,

    /// Time interval when vehicle is allowed to be at this place.
    pub time: TimeInterval,
}

/// Represents a vehicle detail (vehicle shift).
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct VehicleDetail {
    /// A place where vehicle starts.
    pub start: Option<VehiclePlace>,

    /// A place where vehicle ends.
    pub end: Option<VehiclePlace>,
}

/// Represents a vehicle.
pub struct Vehicle {
    /// A vehicle profile.
    pub profile: Profile,

    /// Specifies operating costs for vehicle.
    pub costs: Costs,

    /// Dimensions which contains extra work requirements.
    pub dimens: Dimensions,

    /// Specifies vehicle details.
    pub details: Vec<VehicleDetail>,
}

/// Represents an actor detail: exact start/end location and operating time.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct ActorDetail {
    /// A place where actor's vehicle starts.
    pub start: Option<VehiclePlace>,

    /// A place where actor's vehicle ends.
    pub end: Option<VehiclePlace>,

    /// Time window when actor allowed to work.
    pub time: TimeWindow,
}

/// Represents an actor: abstraction over vehicle and driver.
pub struct Actor {
    /// A vehicle associated within actor.
    pub vehicle: Arc<Vehicle>,

    /// A driver associated within actor.
    pub driver: Arc<Driver>,

    /// Specifies actor detail.
    pub detail: ActorDetail,
}

impl Debug for Actor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("vehicle", &self.vehicle.dimens.get_id().map(|id| id.as_str()).unwrap_or("undef"))
            .finish_non_exhaustive()
    }
}

/// A grouping function for collection of actors.
pub type ActorGroupKeyFn = Box<dyn Fn(&[Arc<Actor>]) -> Box<dyn Fn(&Arc<Actor>) -> usize + Send + Sync>>;

/// Represents available resources to serve jobs.
pub struct Fleet {
    /// All fleet drivers.
    pub drivers: Vec<Arc<Driver>>,

    /// All fleet vehicles.
    pub vehicles: Vec<Arc<Vehicle>>,

    /// All fleet profiles.
    pub profiles: Vec<Profile>,

    /// All fleet actors.
    pub actors: Vec<Arc<Actor>>,

    /// A grouped actors.
    pub groups: HashMap<usize, HashSet<Arc<Actor>>>,
}

impl Fleet {
    /// Creates a new instance of `Fleet`.
    pub fn new(drivers: Vec<Arc<Driver>>, vehicles: Vec<Arc<Vehicle>>, group_key: ActorGroupKeyFn) -> Fleet {
        // TODO we should also consider multiple drivers to support smart vehicle-driver assignment.
        assert_eq!(drivers.len(), 1);
        assert!(!vehicles.is_empty());

        let profiles: HashMap<usize, Profile> = vehicles.iter().map(|v| (v.profile.index, v.profile.clone())).collect();
        let mut profiles = profiles.into_iter().collect::<Vec<_>>();
        profiles.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        let (_, profiles): (Vec<_>, Vec<_>) = profiles.into_iter().unzip();

        let actors = vehicles
            .iter()
            .flat_map(|vehicle| {
                vehicle.details.iter().map(|detail| {
                    Arc::new(Actor {
                        vehicle: vehicle.clone(),
                        driver: drivers.first().unwrap().clone(),
                        detail: ActorDetail {
                            start: detail.start.clone(),
                            end: detail.end.clone(),
                            time: TimeWindow {
                                start: detail.start.as_ref().and_then(|s| s.time.earliest).unwrap_or(0.),
                                end: detail.end.as_ref().and_then(|e| e.time.latest).unwrap_or(f64::MAX),
                            },
                        },
                    })
                })
            })
            .collect::<Vec<_>>();

        let group_key = (*group_key)(&actors);
        let groups = actors.iter().cloned().fold(HashMap::new(), |mut acc, actor| {
            acc.entry((*group_key)(&actor)).or_insert_with(HashSet::new).insert(actor.clone());
            acc
        });

        Fleet { drivers, vehicles, profiles, actors, groups }
    }
}

impl Debug for Fleet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(short_type_name::<Self>())
            .field("vehicles", &self.vehicles.len())
            .field("drivers", &self.drivers.len())
            .field("profiles", &self.profiles.len())
            .field("actors", &self.actors.len())
            .field("groups", &self.groups.len())
            .finish()
    }
}

impl PartialEq<Actor> for Actor {
    fn eq(&self, other: &Actor) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Actor {}

impl Hash for Actor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = self as *const Actor;
        address.hash(state);
    }
}
