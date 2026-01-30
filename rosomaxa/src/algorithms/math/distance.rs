#[cfg(test)]
#[path = "../../../tests/unit/algorithms/math/distance_test.rs"]
mod distance_test;

use crate::{HeuristicSolution, prelude::Float};
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

/// Calculates a distance between two solutions based on their fitness values.
/// Returns the normalized distance in `[0.0, 1.0]`.
pub fn fitness_distance<S>(a: &S, b: &S) -> Float
where
    S: HeuristicSolution,
{
    // Find the first differing fitness component.
    let idx = a
        .fitness()
        .zip(b.fitness())
        .enumerate()
        .find(|(_, (fitness_a, fitness_b))| fitness_a != fitness_b)
        .map(|(idx, _)| idx);

    let idx = match idx {
        Some(idx) => idx,
        None => return 0., // All fitness values equal.
    };

    let total_objectives = a.fitness().count();
    if total_objectives == 0 || total_objectives == idx {
        return 0.;
    }

    // Priority amplifier: earlier objectives matter more.
    let priority_amplifier = (total_objectives - idx) as Float / total_objectives as Float;

    // Relative difference in the differing component.
    let value = a
        .fitness()
        .nth(idx)
        .zip(b.fitness().nth(idx))
        .map(|(a, b)| (a - b).abs() / a.abs().max(b.abs()).max(f64::EPSILON))
        .unwrap_or(0.0);

    value * priority_amplifier
}
