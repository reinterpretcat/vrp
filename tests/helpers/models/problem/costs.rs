use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::{ActivityCost, Driver, TransportCost, Vehicle};
use crate::models::solution::{Activity, Actor};

pub struct TestTransportCost {}

pub struct TestActivityCost {}

impl TransportCost for TestTransportCost {
    fn duration(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Duration {
        subtract(from, to)
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Distance {
        subtract(from, to)
    }
}

impl TestTransportCost {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct ProfileAwareTransportCost {
    func: Box<dyn Fn(Profile, f64) -> f64>,
}

impl ProfileAwareTransportCost {
    pub fn new(func: Box<dyn Fn(Profile, f64) -> f64>) -> ProfileAwareTransportCost {
        ProfileAwareTransportCost { func }
    }
}

impl TransportCost for ProfileAwareTransportCost {
    fn duration(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Duration {
        (self.func)(profile, subtract(from, to))
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, departure: Timestamp) -> Distance {
        (self.func)(profile, subtract(from, to))
    }
}

fn subtract(from: Location, to: Location) -> f64 {
    (if to > from { to - from } else { from - to }) as f64
}

impl ActivityCost for TestActivityCost {}

impl TestActivityCost {
    pub fn new() -> Self {
        Self {}
    }
}
