use crate::models::costs::TransportCost;
use crate::models::solution::Actor;

pub struct TestTransportCost {}

impl TransportCost for TestTransportCost {
    fn cost(&self, actor: &Actor, from: u64, to: u64, departure: f64) -> f64 {
        unimplemented!()
    }

    fn duration(&self, profile: &String, from: u64, to: u64, departure: f64) -> f64 {
        (if to > from { to - from } else { from - to }) as f64
    }

    fn distance(&self, profile: &String, from: u64, to: u64, departure: f64) -> f64 {
        (if to > from { to - from } else { from - to }) as f64
    }
}
