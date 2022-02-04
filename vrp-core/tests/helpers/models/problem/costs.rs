use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{ActivityCost, Actor, SimpleActivityCost, TransportCost, TravelTime};
use crate::models::solution::Activity;
use std::sync::Arc;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration_approx(&self, _: &Profile, from: Location, to: Location) -> Duration {
        fake_routing(from, to)
    }

    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Distance {
        fake_routing(from, to)
    }

    fn duration(&self, _: &Actor, from: Location, to: Location, _: TravelTime) -> Duration {
        fake_routing(from, to)
    }

    fn distance(&self, _: &Actor, from: Location, to: Location, _: TravelTime) -> Distance {
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

#[derive(Default)]
pub struct TestActivityCost {
    inner: SimpleActivityCost,
}

impl ActivityCost for TestActivityCost {
    fn estimate_departure(&self, actor: &Actor, activity: &Activity, arrival: Timestamp) -> Timestamp {
        self.inner.estimate_departure(actor, activity, arrival)
    }

    fn estimate_arrival(&self, actor: &Actor, activity: &Activity, departure: Timestamp) -> Timestamp {
        self.inner.estimate_arrival(actor, activity, departure)
    }
}

impl TestActivityCost {
    pub fn new_shared() -> Arc<dyn ActivityCost + Sync + Send> {
        Arc::new(Self::default())
    }
}
