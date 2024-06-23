use crate::construction::heuristics::StateKeyRegistry;
use crate::solver::HeuristicKeys;
use rosomaxa::prelude::GenericError;
use rustc_hash::FxHasher;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

/// Specifies a type used to store any values regarding problem configuration.
#[derive(Debug)]
pub struct Extras {
    index: HashMap<TypeId, Arc<dyn Any + Send + Sync>, BuildHasherDefault<FxHasher>>,
}

impl Extras {
    /// Gets the value from extras using the key type provided.
    pub fn get_value<K: 'static, V: Send + Sync + 'static>(&self) -> Option<&V> {
        self.index.get(&TypeId::of::<K>()).and_then(|any| any.downcast_ref::<V>())
    }

    /// Gets a shared reference to the value from extras using the key type provided.
    pub fn get_value_raw<K: 'static, V: Send + Sync + 'static>(&self) -> Option<Arc<V>> {
        self.index.get(&TypeId::of::<K>()).cloned().and_then(|any| any.downcast::<V>().ok())
    }

    /// Sets the value to extras using the key type provided.
    pub fn set_value<K: 'static, V: 'static + Sync + Send>(&mut self, value: V) {
        self.index.insert(TypeId::of::<K>(), Arc::new(value));
    }

    /// Sets the value, passed as shared reference, to extras using key type provided.
    pub(crate) fn set_value_raw<K: 'static, V: 'static + Sync + Send>(&mut self, value: Arc<V>) {
        self.index.insert(TypeId::of::<K>(), value);
    }
}

/// Provide the safe way to construct instance of `Extras`.
pub struct ExtrasBuilder(Extras);

impl Default for ExtrasBuilder {
    fn default() -> Self {
        Self::new(&mut StateKeyRegistry::default())
    }
}

impl From<&Extras> for ExtrasBuilder {
    fn from(extras: &Extras) -> Self {
        Self(Extras { index: extras.index.clone() })
    }
}

impl ExtrasBuilder {
    /// Creates an instance of `ExtrasBuilder` using `registry keys` to initialize required keys.
    pub fn new(state_registry: &mut StateKeyRegistry) -> Self {
        let mut builder = Self(Extras { index: Default::default() });

        builder.with_heuristic_keys(HeuristicKeys::from(&mut *state_registry));

        builder
    }

    /// Adds heuristic keys.
    pub fn with_heuristic_keys(&mut self, heuristic_keys: HeuristicKeys) -> &mut Self {
        self.0.set_value::<HeuristicKeys, _>(heuristic_keys);
        self
    }

    /// Adds a custom key-value pair to extras.
    pub fn with_custom_key<K: 'static, T: 'static + Sync + Send>(&mut self, value: Arc<T>) -> &mut Self {
        self.0.set_value_raw::<K, _>(value);
        self
    }

    /// Builds extras.
    pub fn build(&mut self) -> Result<Extras, GenericError> {
        // NOTE: require setting keys below as they are used to calculate important internal
        // metrics such as total cost, rosomaxa weights, etc.

        let error = [(TypeId::of::<HeuristicKeys>(), "heuristic keys needs to be set")]
            .iter()
            .filter(|(key, _)| !self.0.index.contains_key(key))
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
    /// Gets state keys for heuristic.
    fn get_heuristic_keys(&self) -> Option<&HeuristicKeys>;
}

impl CoreStateKeys for Extras {
    fn get_heuristic_keys(&self) -> Option<&HeuristicKeys> {
        self.get_value::<HeuristicKeys, _>()
    }
}
