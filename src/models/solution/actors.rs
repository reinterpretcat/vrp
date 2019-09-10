#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::common::{Location, TimeWindow};
use crate::models::problem::{Driver, Fleet, Vehicle};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Represents actor detail.
#[derive(Clone)]
pub struct Detail {
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
    pub detail: Detail,
}

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<Detail, HashSet<Arc<Actor>>>,
    all: Vec<Arc<Actor>>,
}

impl Registry {
    pub fn new(fleet: &Fleet) -> Registry {
        // TODO we should also consider multiple drivers to support smart vehicle-driver assignment.
        assert_eq!(fleet.drivers.len(), 1);
        assert!(fleet.vehicles.len() > 0);

        let mut available: HashMap<Detail, HashSet<Arc<Actor>>> = Default::default();
        let mut all: Vec<Arc<Actor>> = Default::default();

        for (_, vehicle) in fleet.vehicles.iter().enumerate() {
            for (_, detail) in vehicle.details.iter().enumerate() {
                let actor = Arc::new(Actor {
                    vehicle: vehicle.clone(),
                    driver: fleet.drivers.first().unwrap().clone(),
                    detail: Detail {
                        start: detail.start,
                        end: detail.end,
                        time: detail.time.clone().unwrap_or(TimeWindow { start: 0.0, end: std::f64::MAX }),
                    },
                });
                available.entry(actor.detail.clone()).or_insert(HashSet::new()).insert(actor.clone());
                all.push(actor);
            }
        }

        Registry { available, all }
    }

    /// Removes actor from the list of available actors.
    pub fn use_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(&actor.detail).unwrap().remove(actor);
    }

    /// Adds actor to the list of available actors.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(&actor.detail).unwrap().insert(actor.clone());
    }

    /// Returns all actors.
    pub fn all<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.all.iter().cloned()
    }

    /// Returns list of all available actors.
    pub fn available<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.available.iter().flat_map(|(_, set)| set.into_iter().cloned())
    }

    /// Returns next available actors from each different type.
    pub fn next<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.available.iter().flat_map(|(_, set)| set.into_iter().take(1).cloned())
    }
}

impl PartialEq for Detail {
    fn eq(&self, other: &Self) -> bool {
        other.start == self.start
            && other.end == self.end
            && other.time.start == self.time.start
            && other.time.end == self.time.end
    }
}

impl Eq for Detail {}

impl Hash for Detail {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);

        ((self.time.start * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.time.end * 1024.0 * 1024.0).round() as i64).hash(state);
    }
}

impl PartialEq<Actor> for Actor {
    fn eq(&self, other: &Actor) -> bool {
        &*self as *const Actor == &*other as *const Actor
    }
}

impl Eq for Actor {}

impl Hash for Actor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = &*self as *const Actor;
        address.hash(state);
    }
}
