#[cfg(test)]
#[path = "../../../tests/unit/construction/states/route_test.rs"]
mod route_test;

use crate::models::common::Cost;
use crate::models::solution::{Activity, TourActivity};
use std::any::Any;
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

/// Provides the way to associate arbitrary data within route and activity.
pub struct RouteState {
    route_states: HashMap<i32, Box<dyn Any>>,
    activity_states: HashMap<ActivityWithKey, Box<dyn Any>>,
    keys: HashSet<i32>,
}

impl RouteState {
    /// Creates a new RouteState.
    pub fn new() -> RouteState {
        RouteState::new_with_sizes((2, 4))
    }

    /// Creates a new RouteState using giving capacities.
    pub fn new_with_sizes(sizes: (usize, usize)) -> RouteState {
        RouteState {
            route_states: HashMap::with_capacity(sizes.0),
            activity_states: HashMap::with_capacity(sizes.1),
            keys: Default::default(),
        }
    }

    /// Gets value associated with key.
    pub fn get_route_state<T: 'static>(&self, key: i32) -> Option<&T> {
        self.route_states.get(&key).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_activity_state<T: 'static>(&self, key: i32, activity: &TourActivity) -> Option<&T> {
        self.activity_states.get(&ActivityWithKey(activity.clone(), key)).and_then(|s| s.downcast_ref::<T>())
    }

    /// Puts value associated with key.
    pub fn put_route_state<T: 'static>(&mut self, key: i32, value: T) {
        self.route_states.insert(key, Box::new(value));
        self.keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state<T: 'static>(&mut self, key: i32, activity: &TourActivity, value: T) {
        self.activity_states.insert(ActivityWithKey(activity.clone(), key), Box::new(value));
        self.keys.insert(key);
    }

    /// Removes all activity states for given activity.
    pub fn remove_activity_states(&mut self, activity: &TourActivity) {
        for (_, key) in self.keys.iter().enumerate() {
            self.activity_states.remove(&ActivityWithKey(activity.clone(), *key));
        }
    }

    /// Returns size route state storage.
    pub fn sizes(&self) -> (usize, usize) {
        (self.route_states.capacity(), self.activity_states.capacity())
    }
}

struct ActivityWithKey(TourActivity, i32);

impl PartialEq for ActivityWithKey {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
            && (&*self.0.read().unwrap().deref() as *const Activity)
                == (&*other.0.read().unwrap().deref() as *const Activity)
    }
}

impl Eq for ActivityWithKey {}

impl Hash for ActivityWithKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = &*self.0.read().unwrap().deref() as *const Activity;
        address.hash(state);
        state.write_i32(self.1)
    }
}
