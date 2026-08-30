#[cfg(test)]
#[path = "../../../tests/unit/models/solution/actor_test.rs"]
mod actor_test;

use crate::models::problem::{Actor, Fleet};
use rosomaxa::prelude::Random;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Specifies an entity responsible for providing actors and keeping track of their usage.
pub struct Registry {
    available: HashMap<usize, HashSet<Arc<Actor>>>,
    index: Arc<HashMap<Arc<Actor>, usize>>,
    all: Arc<Vec<Arc<Actor>>>,
    is_scoped: bool,
    random: Arc<dyn Random>,
}

impl Registry {
    /// Creates a new instance of `Registry`
    pub fn new(fleet: &Fleet, random: Arc<dyn Random>) -> Self {
        let index = fleet
            .groups
            .iter()
            .flat_map(|(group_id, actors)| actors.iter().map(|a| (a.clone(), *group_id)).collect::<Vec<_>>())
            .collect();

        Self {
            available: fleet.groups.clone(),
            index: Arc::new(index),
            all: Arc::new(fleet.actors.to_vec()),
            is_scoped: false,
            random,
        }
    }

    /// Removes an actor from the list of available actors.
    /// Returns whether the actor was present in the registry.
    pub fn use_actor(&mut self, actor: &Actor) -> bool {
        self.index.get(actor).and_then(|idx| self.available.get_mut(idx)).is_some_and(|set| set.remove(actor))
    }

    /// Adds actor to the list of available actors.
    /// Returns whether the actor was not present in the registry.
    pub fn free_actor(&mut self, actor: &Arc<Actor>) -> bool {
        if self.is_scoped && !self.all.contains(actor) {
            return false;
        }

        self.index.get(actor).and_then(|idx| self.available.get_mut(idx)).is_some_and(|set| set.insert(actor.clone()))
    }

    /// Returns all actors.
    pub fn all(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
        self.all.iter().cloned()
    }

    /// Returns list of all available actors.
    pub fn available(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
        self.available.values().flat_map(|set| set.iter().cloned())
    }

    /// Returns next available actors from each different type.
    pub fn next(&'_ self) -> impl Iterator<Item = Arc<Actor>> + '_ {
        self.available.values().flat_map(|set| {
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
            is_scoped: self.is_scoped,
            random: self.random.clone(),
        }
    }

    /// Creates a copy in which every actor contained in this registry is available.
    pub(crate) fn deep_copy_with_all_available(&self) -> Self {
        let available = self.all.iter().fold(HashMap::<_, HashSet<_>>::new(), |mut available, actor| {
            available.entry(self.index[actor]).or_default().insert(actor.clone());
            available
        });

        Self {
            available,
            index: self.index.clone(),
            all: self.all.clone(),
            is_scoped: self.is_scoped,
            random: self.random.clone(),
        }
    }

    /// Creates a deep sliced copy of registry keeping only specific actors.
    pub fn deep_slice(&self, filter: impl Fn(&Actor) -> bool) -> Self {
        let all = Arc::new(self.all.iter().filter(|actor| filter(actor.as_ref())).cloned().collect::<Vec<_>>());
        let available = all.iter().fold(HashMap::<_, HashSet<_>>::new(), |mut available, actor| {
            let idx = self.index[actor];
            let group = available.entry(idx).or_default();

            if self.available.get(&idx).is_some_and(|actors| actors.contains(actor)) {
                group.insert(actor.clone());
            }

            available
        });

        Self { available, index: self.index.clone(), all, is_scoped: true, random: self.random.clone() }
    }
}
