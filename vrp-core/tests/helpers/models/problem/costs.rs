use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{ActivityCost, Actor, TransportCost};
use std::sync::Arc;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration_approx(&self, _: &Profile, from: Location, to: Location) -> Duration {
        fake_routing(from, to)
    }

    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Distance {
        fake_routing(from, to)
    }

    fn duration(&self, _: &Actor, from: Location, to: Location, _: Timestamp) -> Duration {
        fake_routing(from, to)
    }

    fn distance(&self, _: &Actor, from: Location, to: Location, _: Timestamp) -> Distance {
        fake_routing(from, to)
    }
}

impl TestTransportCost {
    pub fn new_shared() -> Arc<dyn TransportCost + Sync + Send> {
        Arc::new(Self::default())
    }
}

impl Default for TestTransportCost {
    fn default() -> Self {
        Self {}
    }
}

pub fn fake_routing(from: Location, to: Location) -> f64 {
    (if to > from { to - from } else { from - to }) as f64
}

pub struct TestActivityCost {}

impl ActivityCost for TestActivityCost {}

impl Default for TestActivityCost {
    fn default() -> Self {
        Self {}
    }
}

impl TestActivityCost {
    pub fn new_shared() -> Arc<dyn ActivityCost + Sync + Send> {
        Arc::new(Self::default())
    }
}
