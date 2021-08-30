#[cfg(test)]
#[path = "../../../tests/unit/algorithms/geometry/point_test.rs"]
mod point_test;

use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

/// Represents a point in 2D space.
#[derive(Clone, Debug)]
pub struct Point {
    /// X value.
    pub x: f64,
    /// Y value.
    pub y: f64,
}

impl Point {
    /// Creates a new instance of `Point`.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Computes distance from given point to `other`
    pub fn distance_to_point(&self, other: &Point) -> f64 {
        let delta_x = self.x - other.x;
        let delta_y = self.y - other.y;

        (delta_x * delta_x + delta_y * delta_y).sqrt()
    }

    /// Computes distance from line, drawn by points a and b, to the point.
    pub fn distance_to_line(&self, a: &Point, b: &Point) -> f64 {
        let a_b_distance = a.distance_to_point(b);

        if compare_floats(a_b_distance, 0.) == Ordering::Equal {
            0.
        } else {
            (Self::cross_product(a, b, self) / a_b_distance).abs()
        }
    }

    /// Computes distance from segment to the point.
    pub fn distance_to_segment(&self, a: &Point, b: &Point) -> f64 {
        if Self::dot_product(a, b, self) > 0. {
            return b.distance_to_point(self);
        }

        if Self::dot_product(b, a, self) > 0. {
            return a.distance_to_point(self);
        }

        self.distance_to_line(a, b)
    }

    /// Computes the dot product AB . BC
    pub fn dot_product(a: &Point, b: &Point, c: &Point) -> f64 {
        let ab_x = b.x - a.x;
        let ab_y = b.y - a.y;
        let bc_x = c.x - b.x;
        let bc_y = c.y - b.y;

        ab_x * bc_x + ab_y * bc_y
    }

    /// Computes the cross product AB x AC
    pub fn cross_product(a: &Point, b: &Point, c: &Point) -> f64 {
        let ab_x = b.x - a.x;
        let ab_y = b.y - a.y;
        let ac_x = c.x - a.x;
        let ac_y = c.y - a.y;

        ab_x * ac_y - ab_y * ac_x
    }
}

impl Point {
    fn transmute(&self) -> (i64, i64) {
        let x = self.x.to_bits() as i64;
        let y = self.y.to_bits() as i64;

        (x, y)
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (x, y) = self.transmute();
        x.hash(state);
        y.hash(state);
    }
}

impl Eq for Point {}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        let (self_x, self_y) = self.transmute();
        let (other_x, other_y) = other.transmute();

        self_x == other_x && self_y == other_y
    }
}
