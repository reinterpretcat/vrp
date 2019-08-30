use crate::models::common::{Distance, Duration, Location, Profile, Timestamp};
use crate::models::problem::TransportCost;
use crate::models::solution::Actor;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn duration(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Duration {
        subtract(from, to)
    }

    fn distance(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Distance {
        subtract(from, to)
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
    fn duration(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Duration {
        (self.func)(profile, subtract(from, to))
    }

    fn distance(
        &self,
        profile: Profile,
        from: Location,
        to: Location,
        departure: Timestamp,
    ) -> Distance {
        (self.func)(profile, subtract(from, to))
    }
}

fn subtract(from: Location, to: Location) -> f64 {
    (if to > from { to - from } else { from - to }) as f64
}
