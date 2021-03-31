use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{ActivityCost, TransportCost};
use std::sync::Arc;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration(&self, _: &Profile, from: Location, to: Location, _departure: Timestamp) -> Duration {
        fake_routing(from, to)
    }

    fn distance(&self, _: &Profile, from: Location, to: Location, _departure: Timestamp) -> Distance {
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
