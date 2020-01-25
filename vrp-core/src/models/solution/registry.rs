#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, Fleet};
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<usize, HashSet<Arc<Actor>>>,
    index: HashMap<Arc<Actor>, usize>,
    all: Vec<Arc<Actor>>,
}

impl Registry {
    /// Creates a new instance of [`Registry`];
    pub fn new(fleet: &Fleet) -> Self {
        let index = fleet
            .groups
            .iter()
            .flat_map(|(group_id, actors)| actors.iter().map(|a| (a.clone(), *group_id)).collect::<Vec<_>>())
            .collect();

        Self { available: fleet.groups.clone(), index, all: fleet.actors.to_vec() }
    }

    /// Removes actor from the list of available actors.
    pub fn use_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(self.index.get(actor).unwrap()).unwrap().remove(actor);
    }

    /// Adds actor to the list of available actors.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) {
        self.available.get_mut(self.index.get(actor).unwrap()).unwrap().insert(actor.clone());
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

    /// Creates a deep copy of registry.
    pub fn deep_copy(&self) -> Self {
        Self { available: self.available.clone(), index: self.index.clone(), all: self.all.clone() }
    }
}
