#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Pair(pub usize, pub usize);

#[derive(Debug, Copy, Clone)]
pub struct Triple(pub f64, pub f64, pub f64);

mod objective;
pub use self::objective::*;

impl Eq for Triple {}

impl PartialEq for Triple {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() < std::f64::EPSILON
            && (self.1 - other.1).abs() < std::f64::EPSILON
            && (self.2 - other.2).abs() < std::f64::EPSILON
    }
}
