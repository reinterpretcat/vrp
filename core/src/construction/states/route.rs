#[cfg(test)]
#[path = "../../../tests/unit/construction/states/route_test.rs"]
mod route_test;

use crate::construction::states::OP_START_MSG;
use crate::models::common::Schedule;
use crate::models::problem::Actor;
use crate::models::solution::{Activity, Place, Route, Tour, TourActivity};
use crate::utils::as_mut;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

pub type RouteStateValue = Arc<dyn Any + Send + Sync>;

/// Specifies insertion context for route.
#[derive(Clone)]
pub struct RouteContext {
    /// Used route.
    pub route: Arc<Route>,

    /// Insertion state.
    pub state: Arc<RouteState>,
}

/// Provides the way to associate arbitrary data within route and activity.
pub struct RouteState {
    route_states: HashMap<i32, RouteStateValue>,
    activity_states: HashMap<ActivityWithKey, RouteStateValue>,
    keys: HashSet<i32>,
}

impl RouteContext {
    pub fn as_mut(&mut self) -> (&mut Route, &mut RouteState) {
        let route: &mut Route = unsafe { as_mut(&self.route) };
        let state: &mut RouteState = unsafe { as_mut(&self.state) };

        (route, state)
    }

    pub fn route_mut(&mut self) -> &mut Route {
        unsafe { as_mut(&self.route) }
    }

    pub fn state_mut(&mut self) -> &mut RouteState {
        unsafe { as_mut(&self.state) }
    }
}

impl RouteContext {
    pub fn new(actor: Arc<Actor>) -> Self {
        let mut tour = Tour::default();
        tour.set_start(create_start_activity(&actor));
        create_end_activity(&actor).map(|end| tour.set_end(end));

        RouteContext { route: Arc::new(Route { actor, tour }), state: Arc::new(RouteState::default()) }
    }

    pub fn deep_copy(&self) -> Self {
        let new_route = Route { actor: self.route.actor.clone(), tour: self.route.tour.deep_copy() };
        let mut new_state = RouteState::new_with_sizes(self.state.sizes());

        // copy activity states
        self.route.tour.all_activities().zip(0usize..).for_each(|(a, index)| {
            self.state.all_keys().for_each(|key| {
                if let Some(value) = self.state.get_activity_state_raw(key, a) {
                    let a = new_route.tour.get(index).unwrap();
                    new_state.put_activity_state_raw(key, a, value.clone());
                }
            });
        });

        // copy route states
        self.state.all_keys().for_each(|key| {
            if let Some(value) = self.state.get_route_state_raw(key) {
                new_state.put_route_state_raw(key, value.clone());
            }
        });

        RouteContext { route: Arc::new(new_route), state: Arc::new(new_state) }
    }
}

impl PartialEq<RouteContext> for RouteContext {
    fn eq(&self, other: &RouteContext) -> bool {
        self.route.deref() as *const Route == other.route.deref() as *const Route
    }
}

impl Eq for RouteContext {}

impl Default for RouteState {
    fn default() -> RouteState {
        RouteState::new_with_sizes((2, 4))
    }
}

impl RouteState {
    /// Creates a new RouteState using giving capacities.
    pub fn new_with_sizes(sizes: (usize, usize)) -> RouteState {
        RouteState {
            route_states: HashMap::with_capacity(sizes.0),
            activity_states: HashMap::with_capacity(sizes.1),
            keys: Default::default(),
        }
    }

    /// Gets value associated with key converted to given type.
    pub fn get_route_state<T: Send + Sync + 'static>(&self, key: i32) -> Option<&T> {
        self.route_states.get(&key).and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_route_state_raw(&self, key: i32) -> Option<&RouteStateValue> {
        self.route_states.get(&key)
    }

    /// Gets value associated with key converted to given type.
    pub fn get_activity_state<T: Send + Sync + 'static>(&self, key: i32, activity: &TourActivity) -> Option<&T> {
        self.activity_states
            .get(&(activity.as_ref() as *const Activity as usize, key))
            .and_then(|s| s.downcast_ref::<T>())
    }

    /// Gets value associated with key.
    pub fn get_activity_state_raw(&self, key: i32, activity: &TourActivity) -> Option<&RouteStateValue> {
        self.activity_states.get(&(activity.as_ref() as *const Activity as usize, key))
    }

    /// Puts value associated with key.
    pub fn put_route_state<T: Send + Sync + 'static>(&mut self, key: i32, value: T) {
        self.route_states.insert(key, Arc::new(value));
        self.keys.insert(key);
    }

    /// Puts value associated with key.
    pub fn put_route_state_raw(&mut self, key: i32, value: Arc<dyn Any + Send + Sync>) {
        self.route_states.insert(key, value);
        self.keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state<T: Send + Sync + 'static>(&mut self, key: i32, activity: &TourActivity, value: T) {
        self.activity_states.insert((activity.as_ref() as *const Activity as usize, key), Arc::new(value));
        self.keys.insert(key);
    }

    /// Puts value associated with key and specific activity.
    pub fn put_activity_state_raw(&mut self, key: i32, activity: &TourActivity, value: RouteStateValue) {
        self.activity_states.insert((activity.as_ref() as *const Activity as usize, key), value);
        self.keys.insert(key);
    }

    /// Removes all activity states for given activity.
    pub fn remove_activity_states(&mut self, activity: &TourActivity) {
        for (_, key) in self.keys.iter().enumerate() {
            self.activity_states.remove(&(activity.as_ref() as *const Activity as usize, *key));
        }
    }

    /// Returns all state keys.
    pub fn all_keys<'a>(&'a self) -> impl Iterator<Item = i32> + 'a {
        self.keys.iter().cloned()
    }

    /// Returns size route state storage.
    pub fn sizes(&self) -> (usize, usize) {
        (self.route_states.capacity(), self.activity_states.capacity())
    }
}

type ActivityWithKey = (usize, i32);

pub fn create_start_activity(actor: &Arc<Actor>) -> TourActivity {
    Box::new(Activity {
        place: Place {
            location: actor.detail.start.unwrap_or_else(|| unimplemented!("{}", OP_START_MSG)),
            duration: 0.0,
            time: actor.detail.time.clone(),
        },
        schedule: Schedule { arrival: actor.detail.time.start, departure: actor.detail.time.start },
        job: None,
    })
}

pub fn create_end_activity(actor: &Arc<Actor>) -> Option<TourActivity> {
    actor.detail.end.map(|location| {
        Box::new(Activity {
            place: Place { location, duration: 0.0, time: actor.detail.time.clone() },
            schedule: Schedule { arrival: actor.detail.time.end, departure: actor.detail.time.end },
            job: None,
        })
    })
}
