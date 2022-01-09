pub mod dbscan;
pub mod vicinity;

use crate::algorithms::geometry::Point;

pub fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
