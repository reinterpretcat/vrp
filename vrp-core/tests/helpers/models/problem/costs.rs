use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{ActivityCost, SimpleActivityCost, TransportCost, TravelTime};
use crate::models::solution::{Activity, Route};
use std::sync::Arc;

#[derive(Default)]
pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration_approx(&self, _: &Profile, from: Location, to: Location) -> Duration {
        fake_routing(from, to)
    }

    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Distance {
        fake_routing(from, to)
    }

    fn duration(&self, _: &Route, from: Location, to: Location, _: TravelTime) -> Duration {
        fake_routing(from, to)
    }

    fn distance(&self, _: &Route, from: Location, to: Location, _: TravelTime) -> Distance {
        fake_routing(from, to)
    }
}

impl TestTransportCost {
    pub fn new_shared() -> Arc<dyn TransportCost + Sync + Send> {
        Arc::new(Self::default())
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
    fn estimate_departure(&self, route: &Route, activity: &Activity, arrival: Timestamp) -> Timestamp {
        self.inner.estimate_departure(route, activity, arrival)
    }

    fn estimate_arrival(&self, route: &Route, activity: &Activity, departure: Timestamp) -> Timestamp {
        self.inner.estimate_arrival(route, activity, departure)
    }
}

impl TestActivityCost {
    pub fn new_shared() -> Arc<dyn ActivityCost + Sync + Send> {
        Arc::new(Self::default())
    }
}
