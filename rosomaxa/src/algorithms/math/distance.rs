#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/distance_test.rs"]
mod distance_test;

use crate::prelude::Float;
use std::borrow::Borrow;

/// Calculates relative distance between two vectors. As weights are not normalized, apply
/// standardization using relative change: D = |x - y| / max(|x|, |y|)
pub fn relative_distance<IA, IB>(a: IA, b: IB) -> Float
where
    IA: Iterator,
    IB: Iterator,
    IA::Item: Borrow<Float>,
    IB::Item: Borrow<Float>,
{
    a.zip(b)
        .fold(Float::default(), |acc, (a, b)| {
            let (a, b) = (a.borrow(), b.borrow());
            let divider = a.abs().max(b.abs());
            let change = if divider == 0. { 0. } else { (a - b).abs() / divider };

            acc + change * change
        })
        .sqrt()
}
