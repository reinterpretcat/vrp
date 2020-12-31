#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, Fleet};
use crate::utils::Random;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<usize, HashSet<Arc<Actor>>>,
    index: HashMap<Arc<Actor>, usize>,
    all: Vec<Arc<Actor>>,
    random: Arc<dyn Random + Send + Sync>,
}

impl Registry {
    /// Creates a new instance of `Registry`
    pub fn new(fleet: &Fleet, random: Arc<dyn Random + Send + Sync>) -> Self {
        let index = fleet
            .groups
            .iter()
            .flat_map(|(group_id, actors)| actors.iter().map(|a| (a.clone(), *group_id)).collect::<Vec<_>>())
            .collect();

        Self { available: fleet.groups.clone(), index, all: fleet.actors.to_vec(), random }
    }

    /// Removes an actor from the list of available actors.
    /// Returns whether the actor was present in the registry.
    pub fn use_actor(&mut self, actor: &Arc<Actor>) -> bool {
        self.available.get_mut(self.index.get(actor).unwrap()).unwrap().remove(actor)
    }

    /// Adds actor to the list of available actors.
    /// Returns whether the actor was not present in the registry.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) -> bool {
        self.available.get_mut(self.index.get(actor).unwrap()).unwrap().insert(actor.clone())
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
        self.available.iter().flat_map(move |(_, set)| {
            // NOTE pick a random actor from set of available actors.
            let skip_amount = if set.len() < 2 { 0 } else { self.random.uniform_int(0, set.len() as i32 - 1) as usize };
            set.iter().skip(skip_amount).take(1).cloned()
        })
    }

    /// Creates a deep copy of registry.
    pub fn deep_copy(&self) -> Self {
        Self {
            available: self.available.clone(),
            index: self.index.clone(),
            all: self.all.clone(),
            random: self.random.clone(),
        }
    }

    /// Creates a deep sliced copy of registry keeping only specific actors.
    pub fn deep_slice(&self, filter: impl Fn(&Actor) -> bool) -> Self {
        Self {
            available: self
                .available
                .iter()
                .filter_map(|(idx, actors)| {
                    let actors = actors.iter().filter(|actor| filter(actor.as_ref())).cloned().collect::<HashSet<_>>();
                    if actors.is_empty() {
                        None
                    } else {
                        Some((*idx, actors))
                    }
                })
                .collect(),
            index: self
                .index
                .iter()
                .filter(|(actor, _)| filter(actor.as_ref()))
                .map(|(actor, idx)| (actor.clone(), *idx))
                .collect(),
            all: self.all.iter().filter(|actor| filter(actor.as_ref())).cloned().collect(),
            random: self.random.clone(),
        }
    }
}
