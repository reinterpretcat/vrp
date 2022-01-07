use crate::utils::compare_floats;
use std::cmp::Ordering;

/// Calculates relative distance between two vectors. As weights are not normalized, apply
/// standardization using relative change: D = |x - y| / max(|x|, |y|)
pub fn relative_distance<A, B>(a: A, b: B) -> f64
where
    A: Iterator<Item = f64>,
    B: Iterator<Item = f64>,
{
    a.zip(b)
        .fold(0_f64, |acc, (a, b)| {
            let divider = a.abs().max(b.abs());
            let change = if compare_floats(divider, 0.) == Ordering::Equal { 0. } else { (a - b) / divider };

            acc + change * change
        })
        .sqrt()
}
