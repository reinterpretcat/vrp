use crate::construction::enablers::ScheduleKeys;
use crate::construction::features::{CapacityDimenKeys, CapacityKeys, CapacityStateKeys};
use crate::construction::heuristics::StateKeyRegistry;
use crate::models::common::DimenKeyRegistry;
use crate::solver::HeuristicKeys;
use hashbrown::HashMap;
use rosomaxa::prelude::GenericError;
use rustc_hash::FxHasher;
use std::any::Any;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

/// Specifies a type used to store any values regarding problem configuration.
pub struct Extras {
    index: HashMap<String, Arc<dyn Any + Send + Sync>, BuildHasherDefault<FxHasher>>,
}

impl Extras {
    /// Returns a shared reference for the value under the given key.
    pub fn get_value_raw<T: 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.index.get(key).cloned().and_then(|any| any.downcast::<T>().ok())
    }
}

/// Provide the safe way to construct instance of `Extras`.
pub struct ExtrasBuilder(Extras);

impl Default for ExtrasBuilder {
    fn default() -> Self {
        Self::new(&mut DimenKeyRegistry::default(), &mut StateKeyRegistry::default())
    }
}

impl From<&Extras> for ExtrasBuilder {
    fn from(extras: &Extras) -> Self {
        Self(Extras { index: extras.index.clone() })
    }
}

impl ExtrasBuilder {
    /// Creates an instance of `ExtrasBuilder` using `registry` to initialize required keys.
    pub fn new(dimen_registry: &mut DimenKeyRegistry, state_registry: &mut StateKeyRegistry) -> Self {
        let mut builder = Self(Extras { index: Default::default() });

        builder
            .with_schedule_keys(ScheduleKeys::from(&mut *state_registry))
            .with_capacity_keys(CapacityKeys {
                state_keys: CapacityStateKeys::from(&mut *state_registry),
                dimen_keys: CapacityDimenKeys::from(&mut *dimen_registry),
            })
            .with_heuristic_keys(HeuristicKeys::from(&mut *state_registry));

        builder
    }

    /// Adds schedule keys.
    pub fn with_schedule_keys(&mut self, schedule_keys: ScheduleKeys) -> &mut Self {
        self.0.set_value("schedule_keys", schedule_keys);
        self
    }

    /// Adds capacity keys.
    pub fn with_capacity_keys(&mut self, capacity_keys: CapacityKeys) -> &mut Self {
        self.0.set_value("capacity_keys", capacity_keys);
        self
    }

    /// Adds heuristic keys.
    pub fn with_heuristic_keys(&mut self, heuristic_keys: HeuristicKeys) -> &mut Self {
        self.0.set_value("heuristic_keys", heuristic_keys);
        self
    }

    /// Adds a custom key-value pair to extras.
    pub fn with_custom_key<T: 'static + Sync + Send>(&mut self, key: &str, value: Arc<T>) -> &mut Self {
        self.0.index.insert(key.to_string(), value);
        self
    }

    /// Builds extras.
    pub fn build(&mut self) -> Result<Extras, GenericError> {
        // NOTE: require setting keys below as they are used to calculate important internal
        // metrics such as total cost, rosomaxa weights, etc.

        let error =
            [("schedule_keys", "schedule keys needs to be set"), ("heuristic_keys", "heuristic keys needs to be set")]
                .iter()
                .filter(|(key, _)| !self.0.index.contains_key(*key))
                .map(|(_, msg)| GenericError::from(*msg))
                .next();

        if let Some(error) = error {
            return Err(error);
        }

        Ok(Extras { index: std::mem::take(&mut self.0.index) })
    }
}

/// Keeps track of state keys used in core which can be useful in other contexts too.
///
/// For example, transport keys provide information about total duration/distance traveled by vehicle
/// which can be used in heuristic context to compare different routes.
pub trait CoreStateKeys {
    /// Get state keys for scheduling.
    fn get_schedule_keys(&self) -> Option<&ScheduleKeys>;

    /// Gets state keys for capacity feature.
    fn get_capacity_keys(&self) -> Option<&CapacityKeys>;

    /// Gets state keys for heuristic.
    fn get_heuristic_keys(&self) -> Option<&HeuristicKeys>;
}

impl CoreStateKeys for Extras {
    fn get_schedule_keys(&self) -> Option<&ScheduleKeys> {
        self.get_value("schedule_keys")
    }

    fn get_capacity_keys(&self) -> Option<&CapacityKeys> {
        self.get_value("capacity_keys")
    }

    fn get_heuristic_keys(&self) -> Option<&HeuristicKeys> {
        self.get_value("heuristic_keys")
    }
}

impl Extras {
    /// Gets value from index.
    pub fn get_value<T: 'static>(&self, key: &str) -> Option<&T> {
        self.index.get(key).and_then(|any| any.downcast_ref::<T>())
    }

    /// Sets value to index.
    pub fn set_value<T: 'static + Sync + Send>(&mut self, key: &str, value: T) {
        self.index.insert(key.to_owned(), Arc::new(value));
    }
}
