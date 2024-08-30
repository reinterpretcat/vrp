pub mod dbscan;
pub mod vicinity;

use crate::algorithms::geometry::Point;
use rosomaxa::prelude::Float;

pub fn p(x: Float, y: Float) -> Point {
    Point { x, y }
}
