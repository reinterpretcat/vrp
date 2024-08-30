#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/distance_test.rs"]
mod distance_test;

use crate::prelude::Float;
use crate::utils::compare_floats;
use std::cmp::Ordering;

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
            let change = if compare_floats(divider, 0.) == Ordering::Equal { 0. } else { (a - b).abs() / divider };

            acc + change * change
        })
        .sqrt()
}
