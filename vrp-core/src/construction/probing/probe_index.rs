use crate::construction::heuristics::InsertionResult;
use crate::models::problem::Actor;
use crate::prelude::{ConstraintViolation, RouteContext, RouteState};
use std::collections::HashMap;
use std::sync::Arc;

custom_tour_state!(ProbeData typeof ProbeData);

/// Keeps track of internal probing data to speed up evaluations.
/// By default, it is empty and filled during job evaluation into a specific route.
#[derive(Debug, Default)]
pub struct ProbeData {
    data: HashMap<Arc<Actor>, RouteProbeData>,
}

impl ProbeData {
    /// Takes the best probe data from two probes cleaning their state.
    pub(crate) fn merge(left: &mut Self, right: &mut Self) -> Self {
        let data = if left.data.len() > right.data.len() {
            left.data.extend(right.data.drain());
            std::mem::take(&mut left.data)
        } else {
            right.data.extend(left.data.drain());
            std::mem::take(&mut right.data)
        };

        Self { data }
    }

    pub(crate) fn remove(&mut self, actor: &Arc<Actor>) {
        self.data.remove(actor);
    }

    pub(crate) fn extend(&mut self, other: &Self) {
        self.data.extend(other.data.iter().map(|(key, value)| (key.clone(), value.clone())));
    }
}

/// Keeps track of route specific probing data.
#[derive(Clone, Debug, Default)]
pub(crate) struct RouteProbeData {
    actor: Option<Arc<Actor>>,
    data: HashMap<(usize, usize), (ConstraintViolation, bool)>,
}

/// Probe index keeps track of precalculated probing data (e.g., on previous iterations) and a dynamic one.
#[derive(Debug)]
pub(crate) struct RouteProbeIndex<'a> {
    shared: Option<&'a RouteProbeData>,
    dynamic: RouteProbeData,
}

impl<'a> RouteProbeIndex<'a> {
    pub fn get(&self, key: &(usize, usize)) -> Option<&(ConstraintViolation, bool)> {
        self.shared.and_then(|pd| pd.data.get(key))
    }

    pub fn insert(&mut self, key: (usize, usize), value: (ConstraintViolation, bool)) {
        self.dynamic.data.insert(key, value);
    }
}

impl<'a> From<&'a RouteContext> for RouteProbeIndex<'a> {
    fn from(route_ctx: &'a RouteContext) -> Self {
        Self {
            shared: route_ctx.state().get_probe_data().and_then(|probe| probe.data.get(&route_ctx.route().actor)),
            dynamic: RouteProbeData { actor: Some(route_ctx.route().actor.clone()), data: Default::default() },
        }
    }
}

impl<'a> From<&'a mut [InsertionResult]> for ProbeData {
    fn from(results: &'a mut [InsertionResult]) -> Self {
        results
            .iter_mut()
            .fold(ProbeData::default(), |mut acc, result| ProbeData::merge(&mut acc, result.get_probe_data_mut()))
    }
}

impl<'a> From<RouteProbeIndex<'a>> for ProbeData {
    fn from(index: RouteProbeIndex<'a>) -> Self {
        let Some(actor) = index.dynamic.actor.clone() else {
            return Self::default();
        };

        Self { data: HashMap::from([(actor, index.dynamic)]) }
    }
}
