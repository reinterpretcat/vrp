use crate::models::common::Profile;
use crate::models::costs::TransportCost;
use crate::models::solution::Actor;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn cost(&self, actor: &Actor, from: u64, to: u64, departure: f64) -> f64 {
        subtract(from, to)
    }

    fn duration(&self, profile: Profile, from: u64, to: u64, departure: f64) -> f64 {
        subtract(from, to)
    }

    fn distance(&self, profile: Profile, from: u64, to: u64, departure: f64) -> f64 {
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
    fn cost(&self, actor: &Actor, from: u64, to: u64, departure: f64) -> f64 {
        (self.func)(0, subtract(from, to))
    }

    fn duration(&self, profile: Profile, from: u64, to: u64, departure: f64) -> f64 {
        (self.func)(profile, subtract(from, to))
    }

    fn distance(&self, profile: Profile, from: u64, to: u64, departure: f64) -> f64 {
        (self.func)(profile, subtract(from, to))
    }
}

fn subtract(from: u64, to: u64) -> f64 {
    (if to > from { to - from } else { from - to }) as f64
}
