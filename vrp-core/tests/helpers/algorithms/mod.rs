use crate::algorithms::geometry::Point;

pub mod nsga2;

pub fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
