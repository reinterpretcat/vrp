#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/distance_test.rs"]
mod distance_test;

use crate::prelude::Float;

/// Calculates relative distance between two vectors. As weights are not normalized, apply
/// standardization using relative change: D = |x - y| / max(|x|, |y|)
pub fn relative_distance<A, B>(a: A, b: B) -> Float
where
    A: Iterator<Item = Float>,
    B: Iterator<Item = Float>,
{
    a.zip(b)
        .fold(Float::default(), |acc, (a, b)| {
            let divider = a.abs().max(b.abs());
            let change = if divider == 0. { 0. } else { (a - b).abs() / divider };

            acc + change * change
        })
        .sqrt()
}
