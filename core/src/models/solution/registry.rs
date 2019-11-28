#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, ActorDetail, Costs, Fleet};
use hashbrown::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<ActorKey, HashSet<Arc<Actor>>>,
    all: Vec<Arc<Actor>>,
}

impl Registry {
    pub fn new(fleet: &Fleet) -> Registry {
        Registry {
            available: fleet.actors.iter().cloned().fold(HashMap::new(), |mut acc, actor| {
                acc.entry(ActorKey::new(&actor)).or_insert_with(HashSet::new).insert(actor.clone());
                acc
            }),
            all: fleet.actors.to_vec(),
        }
    }

    /// Removes actor from the list of available actors.
    pub fn use_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(&ActorKey::new(&actor)).unwrap().remove(actor);
    }

    /// Adds actor to the list of available actors.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(&ActorKey::new(&actor)).unwrap().insert(actor.clone());
    }

    /// Returns all actors.
    pub fn all<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.all.iter().cloned()
    }

    /// Returns list of all available actors.
    pub fn available<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.available.iter().flat_map(|(_, set)| set.iter().cloned())
    }

    /// Returns next available actors from each different type.
    pub fn next<'a>(&'a self) -> impl Iterator<Item = Arc<Actor>> + 'a {
        self.available.iter().flat_map(|(_, set)| set.iter().take(1).cloned())
    }

    /// Creates a copy of registry
    pub fn deep_copy(&self) -> Registry {
        Self { available: self.available.clone(), all: self.all.clone() }
    }
}

#[derive(Clone)]
struct ActorKey {
    detail: ActorDetail,
    driver_costs: Costs,
    vehicle_costs: Costs,
}

impl ActorKey {
    pub fn new(actor: &Actor) -> Self {
        Self {
            detail: actor.detail.clone(),
            driver_costs: actor.driver.costs.clone(),
            vehicle_costs: actor.vehicle.costs.clone(),
        }
    }

    pub fn same_costs(&self, other: &Self) -> bool {
        Self::compare_costs(&self.vehicle_costs, &other.vehicle_costs)
            && Self::compare_costs(&self.driver_costs, &other.driver_costs)
    }

    fn compare_costs(lhs: &Costs, rhs: &Costs) -> bool {
        lhs.fixed == rhs.fixed
            && lhs.per_distance == rhs.per_distance
            && lhs.per_driving_time == rhs.per_driving_time
            && lhs.per_service_time == rhs.per_service_time
            && lhs.per_waiting_time == rhs.per_waiting_time
    }
}

impl PartialEq for ActorKey {
    fn eq(&self, other: &Self) -> bool {
        other.same_costs(&self)
            && other.detail.start == self.detail.start
            && other.detail.end == self.detail.end
            && other.detail.time.start == self.detail.time.start
            && other.detail.time.end == self.detail.time.end
    }
}

impl Eq for ActorKey {}

impl Hash for ActorKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.detail.start.hash(state);
        self.detail.end.hash(state);

        ((self.detail.time.start * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.detail.time.end * 1024.0 * 1024.0).round() as i64).hash(state);

        ((self.vehicle_costs.fixed * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.vehicle_costs.per_distance * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.vehicle_costs.per_driving_time * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.vehicle_costs.per_service_time * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.vehicle_costs.per_waiting_time * 1024.0 * 1024.0).round() as i64).hash(state);

        ((self.driver_costs.fixed * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.driver_costs.per_distance * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.driver_costs.per_driving_time * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.driver_costs.per_service_time * 1024.0 * 1024.0).round() as i64).hash(state);
        ((self.driver_costs.per_waiting_time * 1024.0 * 1024.0).round() as i64).hash(state);
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
