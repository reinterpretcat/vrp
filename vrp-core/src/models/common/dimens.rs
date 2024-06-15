use hashbrown::HashMap;
use rustc_hash::FxHasher;
use std::any::{Any, TypeId};
use std::hash::BuildHasherDefault;
use std::sync::Arc;

/// Multiple named dimensions which can contain anything:
/// * unit of measure, e.g. volume, mass, size, etc.
/// * set of skills
/// * tag.
#[derive(Clone, Debug, Default)]
pub struct Dimensions {
    index: HashMap<TypeId, Arc<dyn Any + Send + Sync>, BuildHasherDefault<FxHasher>>,
}

impl Dimensions {
    /// Gets a value using key type provided.
    pub fn get_value<K: 'static, V: 'static>(&self) -> Option<&V> {
        self.index.get(&TypeId::of::<K>()).and_then(|any| any.downcast_ref::<V>())
    }

    /// Sets the value using key type provided.
    pub fn set_value<K: 'static, V: 'static + Sync + Send>(&mut self, value: V) {
        self.index.insert(TypeId::of::<K>(), Arc::new(value));
    }
}
