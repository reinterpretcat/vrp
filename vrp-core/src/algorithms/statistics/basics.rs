use crate::utils::compare_floats;
use std::cmp::Ordering;
use std::cmp::Ordering::Equal;

/// Returns coefficient variation.
pub fn get_cv(values: &[f64]) -> f64 {
    let (variance, mean) = get_variance_mean(values);
    if compare_floats(mean, 0.) == Equal {
        return 0.;
    }
    let sdev = variance.sqrt();

    sdev / mean
}

/// Gets mean of values.
pub fn get_mean(values: &[f64]) -> f64 {
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

/// Returns variance.
pub fn get_variance(values: &[f64]) -> f64 {
    get_variance_mean(values).0
}

/// Returns standard deviation.
pub fn get_stdev(values: &[f64]) -> f64 {
    get_variance_mean(values).0.sqrt()
}

/// Returns variance and mean.
fn get_variance_mean(values: &[f64]) -> (f64, f64) {
    let mean = get_mean(values);

    let (first, second) = values.iter().fold((0., 0.), |acc, v| {
        let dev = v - mean;
        (acc.0 + dev * dev, acc.1 + dev)
    });

    // NOTE Bessel's correction is not used here
    ((first - (second * second / values.len() as f64)) / (values.len() as f64), mean)
}

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
