#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, ActorDetail, Fleet};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<ActorDetail, HashSet<Arc<Actor>>>,
    all: Vec<Arc<Actor>>,
}

impl Registry {
    pub fn new(fleet: &Fleet) -> Registry {
        Registry {
            available: fleet.actors.iter().cloned().fold(HashMap::new(), |mut acc, actor| {
                acc.entry(actor.detail.clone()).or_insert(HashSet::new()).insert(actor.clone());
                acc
            }),
            all: fleet.actors.iter().cloned().collect(),
        }
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

    /// Creates a copy of registry
    pub fn deep_copy(&self) -> Registry {
        Self { available: self.available.clone(), all: self.all.clone() }
    }
}

impl PartialEq for ActorDetail {
    fn eq(&self, other: &Self) -> bool {
        other.start == self.start
            && other.end == self.end
            && other.time.start == self.time.start
            && other.time.end == self.time.end
    }
}

impl Eq for ActorDetail {}

impl Hash for ActorDetail {
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
