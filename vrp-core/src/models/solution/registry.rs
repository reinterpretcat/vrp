#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, Fleet};
use hashbrown::{HashMap, HashSet};
use rosomaxa::prelude::Random;
use rosomaxa::utils::DefaultRandom;
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<usize, HashSet<Arc<Actor>>>,
    index: HashMap<Arc<Actor>, usize>,
    all: Vec<Arc<Actor>>,
    random: DefaultRandom,
}

impl Registry {
    /// Creates a new instance of `Registry`
    pub fn new(fleet: &Fleet, random: DefaultRandom) -> Self {
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
        self.available.get_mut(self.index.get(actor).expect("unknown actor")).unwrap().remove(actor)
    }

    /// Adds actor to the list of available actors.
    /// Returns whether the actor was not present in the registry.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) -> bool {
        self.available.get_mut(self.index.get(actor).expect("unknown actor")).unwrap().insert(actor.clone())
    }

    /// Returns all actors.
    pub fn all(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
        self.all.iter().cloned()
    }

    /// Returns list of all available actors.
    pub fn available(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
        self.available.iter().flat_map(|(_, set)| set.iter().cloned())
    }

    /// Returns next available actors from each different type.
    pub fn next(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
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
                .map(|(idx, actors)| {
                    let actors = actors.iter().filter(|actor| filter(actor.as_ref())).cloned().collect::<HashSet<_>>();
                    (*idx, actors)
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
