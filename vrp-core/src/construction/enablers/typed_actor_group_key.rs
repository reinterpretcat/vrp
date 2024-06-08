use crate::models::problem::Actor;
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

/// An actor group key implementation which creates groups using "type" dimension.
pub fn create_typed_actor_groups<F>(actors: &[Arc<Actor>], actor_type_fn: F) -> impl Fn(&Actor) -> usize + Send + Sync
where
    F: Fn(&Actor) -> String,
{
    let unique_type_keys: HashSet<_> = actors.iter().map(|a| (actor_type_fn(a.as_ref()), a.detail.clone())).collect();

    let type_key_map: HashMap<_, _> = unique_type_keys.into_iter().zip(0_usize..).collect();

    let groups: HashMap<_, _> = actors
        .iter()
        .map(|a| (a.clone(), *type_key_map.get(&(actor_type_fn(a.as_ref()), a.detail.clone())).unwrap()))
        .collect();

    move |a| *groups.get(a).unwrap()
}
